//! The default browser, for `auth login`, `uc dev` and `uc web`.

/// Open `url` in the default browser; the caller prints it as well.
pub fn open(url: &str) {
    let mut command = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
    } else if cfg!(target_os = "windows") {
        // Not `cmd /C start`: cmd reads the `&` between query parameters
        // as a command separator and cuts the URL.
        let mut c = std::process::Command::new("rundll32");
        c.arg("url.dll,FileProtocolHandler");
        c
    } else {
        std::process::Command::new("xdg-open")
    };
    // Not opening it is fine: the URL is printed.
    let Ok(mut child) = command
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    else {
        return;
    };
    // Do not wait here: without a desktop, xdg-open can run a text
    // browser that only exits when the user quits it.
    std::thread::spawn(move || child.wait());
}
