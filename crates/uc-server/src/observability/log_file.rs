//! The log file, as upstream's (its `server.log4j2.properties`):
//! `server.log` in the `log.dir` folder, with lines `HH:mm:ss.SSS [thread]
//! LEVEL target - message`, rolled at 10 MB into
//! `server-MM-dd-yyyy-HH-mm-ss-N.log.gz`, keeping the newest 5.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, format::Writer};
use tracing_subscriber::registry::LookupSpan;

const MAX_SIZE: u64 = 10 * 1024 * 1024;
const KEEP: usize = 5;

/// `server.log` in a directory, rolled by size.
pub struct RollingFile {
    max_size: u64,
    state: Mutex<Open>,
}

struct Open {
    dir: PathBuf,
    file: File,
    size: u64,
}

fn open(dir: &Path) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("server.log"))
}

impl RollingFile {
    pub fn new(dir: &Path) -> io::Result<Self> {
        Self::with_max_size(dir, MAX_SIZE)
    }

    pub fn with_max_size(dir: &Path, max_size: u64) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let file = open(dir)?;
        let size = file.metadata()?.len();
        Ok(Self {
            max_size,
            state: Mutex::new(Open {
                dir: dir.to_path_buf(),
                file,
                size,
            }),
        })
    }
}

impl Open {
    /// Compress `server.log` into the next `server-<time>-<n>.log.gz` and
    /// drop all but the newest archives.
    fn roll(&mut self) -> io::Result<()> {
        self.file.flush()?;
        let stamp = jiff::Zoned::now().strftime("%m-%d-%Y-%H-%M-%S");
        let mut n = 1;
        let archive = loop {
            let candidate = self.dir.join(format!("server-{stamp}-{n}.log.gz"));
            if !candidate.exists() {
                break candidate;
            }
            n += 1;
        };
        let current = self.dir.join("server.log");
        let mut encoder =
            flate2::write::GzEncoder::new(File::create(&archive)?, flate2::Compression::default());
        io::copy(&mut File::open(&current)?, &mut encoder)?;
        encoder.finish()?;
        self.file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&current)?;
        self.size = 0;
        let mut archives: Vec<(std::time::SystemTime, PathBuf)> = fs::read_dir(&self.dir)?
            .filter_map(Result::ok)
            .filter(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                name.starts_with("server-") && name.ends_with(".log.gz")
            })
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .collect();
        archives.sort();
        let excess = archives.len().saturating_sub(KEEP);
        for (_, path) in archives.into_iter().take(excess) {
            let _ = fs::remove_file(path);
        }
        Ok(())
    }
}

impl Write for &RollingFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut open = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if open.size > 0 && open.size + buf.len() as u64 > self.max_size {
            open.roll()?;
        }
        open.file.write_all(buf)?;
        open.size += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .file
            .flush()
    }
}

/// Upstream's pattern: `%d{HH:mm:ss.SSS} [%t] %-5level %logger{36} - %msg%n`.
pub struct UpstreamFormat;

impl<S, N> FormatEvent<S, N> for UpstreamFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let meta = event.metadata();
        let thread = std::thread::current();
        let thread = thread
            .name()
            .map(str::to_string)
            .unwrap_or_else(|| format!("{:?}", thread.id()));
        write!(
            writer,
            "{} [{thread}] {:<5} {} - ",
            jiff::Zoned::now().strftime("%H:%M:%S%.3f"),
            meta.level(),
            meta.target()
        )?;
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolls_by_size_and_keeps_five() {
        let dir = tempfile::tempdir().unwrap();
        let log = RollingFile::with_max_size(dir.path(), 100).unwrap();
        for i in 0..40 {
            writeln!(&log, "line {i:02} of the log, long enough to fill it").unwrap();
        }
        let names: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"server.log".to_string()));
        let archives = names
            .iter()
            .filter(|n| n.starts_with("server-") && n.ends_with(".log.gz"))
            .count();
        assert_eq!(archives, 5, "{names:?}");
        assert!(
            fs::read_to_string(dir.path().join("server.log"))
                .unwrap()
                .contains("line 39")
        );
    }
}
