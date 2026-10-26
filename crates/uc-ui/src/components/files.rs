//! The files of a volume or table, through the Files API: the folders of
//! the path over the list, the entries of a folder a page at a time, and a
//! link that downloads each file. The owner of a volume can also upload
//! files, make folders, and delete files and folders. In object storage a
//! folder without files is not there, so it shows as empty.

use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use uc_client::models::files::v1::DirectoryEntry;
use uc_common::FILES_BASE;
use wasm_bindgen_futures::JsFuture;

use super::delete_modal::HoldsChoice;
use super::field::TextField;
use super::icons::{Download, File, Folder, FolderPlus, RefreshCw, Trash2, Upload};
use super::{Card, Modal, loaded_list, use_notify};
use crate::api;
use crate::load::Load;
use crate::util::{format_size, format_timestamp};

/// The entries of a page.
const PAGE_SIZE: i32 = 200;

/// What a URL path keeps of a file's path: the unreserved bytes and `/`.
const PATH: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~')
    .remove(b'/');

#[component]
pub fn FilesCard(
    /// The path of the volume or table, such as
    /// `/Volumes/main/default/landing`.
    root: String,
    /// The caller may upload, make folders and delete: the owner of a
    /// volume.
    #[prop(into, optional)]
    writable: Signal<bool>,
) -> impl IntoView {
    let root = StoredValue::new(root);
    // The folders under the root, to the folder on show.
    let dir = RwSignal::new(Vec::<String>::new());
    let folder = Signal::derive(move || join(&root.get_value(), &dir.get()));
    let entries = RwSignal::new(Load::<Vec<DirectoryEntry>>::Loading);
    // The token of the next page, and whether it loads.
    let next = RwSignal::new(None::<String>);
    let more = RwSignal::new(false);
    // Each load has a number; an answer to an older one is dropped.
    let loads = StoredValue::new(0_u64);

    let load = move |token: Option<String>| {
        let id = loads.get_value() + 1;
        loads.set_value(id);
        let at_root = dir.with_untracked(Vec::is_empty);
        let path = folder.get_untracked();
        let first = token.is_none();
        match first {
            true => entries.set(Load::Loading),
            false => more.set(true),
        }
        spawn_local(async move {
            let page = api::client()
                .list_directory_contents(path)
                .with_page_size(PAGE_SIZE)
                .with_page_token(token)
                .await;
            if loads.try_get_value() != Some(id) {
                return;
            }
            let _ = more.try_set(false);
            match page {
                Ok(page) => {
                    let _ = next.try_set(page.next_page_token);
                    let _ = entries.try_update(|loaded| match loaded {
                        Load::Ready(list) if !first => list.extend(page.contents),
                        _ => *loaded = Load::Ready(page.contents),
                    });
                }
                Err(e) => {
                    let _ = next.try_set(None);
                    let _ = entries.try_set(match e.status() {
                        Some(404) if !at_root => Load::Ready(Vec::new()),
                        _ => Load::Failed(api::failure(&e, "Failed to list the files")),
                    });
                }
            }
        });
    };
    Effect::new(move |_| {
        dir.track();
        load(None);
    });

    let upload_open = RwSignal::new(false);
    let folder_open = RwSignal::new(false);
    let deleting = RwSignal::new(None::<DirectoryEntry>);

    let actions = view! {
        <button
            type="button"
            class="uc-btn uc-btn--ghost uc-btn--icon uc-btn--sm"
            title="Reload"
            aria-label="Reload"
            on:click=move |_| load(None)
        >
            <RefreshCw/>
        </button>
        {move || {
            writable
                .get()
                .then(|| {
                    view! {
                        <button
                            type="button"
                            class="uc-btn uc-btn--outline uc-btn--sm"
                            on:click=move |_| folder_open.set(true)
                        >
                            <FolderPlus/>
                            "New folder"
                        </button>
                        <button
                            type="button"
                            class="uc-btn uc-btn--sm"
                            on:click=move |_| upload_open.set(true)
                        >
                            <Upload/>
                            "Upload"
                        </button>
                    }
                })
        }}
    }
    .into_any();

    let crumbs = move || {
        let name = root.with_value(|r| r.rsplit('/').next().unwrap_or_default().to_string());
        let parts = dir.get();
        let last = parts.len();
        std::iter::once(name)
            .chain(parts)
            .enumerate()
            .map(|(depth, label)| {
                let separator =
                    (depth > 0).then(|| view! { <span class="uc-files__sep">"/"</span> });
                let item = match depth == last {
                    true => view! { <span class="uc-files__here">{label}</span> }.into_any(),
                    false => view! {
                        <button
                            type="button"
                            class="uc-btn uc-btn--link"
                            on:click=move |_| dir.update(|d| d.truncate(depth))
                        >
                            {label}
                        </button>
                    }
                    .into_any(),
                };
                view! {
                    {separator}
                    {item}
                }
            })
            .collect_view()
    };

    let row = move |entry: DirectoryEntry| {
        let name = entry.name.clone();
        let label = match entry.is_directory {
            true => {
                let child = name.clone();
                view! {
                    <button
                        type="button"
                        class="uc-btn uc-btn--link"
                        on:click=move |_| dir.update(|d| d.push(child.clone()))
                    >
                        <Folder class="uc-icon--volume"/>
                        {name.clone()}
                    </button>
                }
                .into_any()
            }
            false => view! {
                <span class="uc-files__name">
                    <File/>
                    {name.clone()}
                </span>
            }
            .into_any(),
        };
        let size = entry.file_size.map_or_else(|| "—".to_string(), format_size);
        let modified = format_timestamp(entry.last_modified.unwrap_or_default());
        let download = (!entry.is_directory).then(|| {
            let href = format!(
                "{FILES_BASE}/files{}",
                utf8_percent_encode(&entry.path, PATH)
            );
            view! {
                <a
                    class="uc-btn uc-btn--ghost uc-btn--icon uc-btn--xs"
                    href=href
                    download=name.clone()
                    title="Download"
                    aria-label=format!("Download {name}")
                >
                    <Download/>
                </a>
            }
        });
        let delete = move || {
            writable.get().then(|| {
                let target = entry.clone();
                view! {
                    <button
                        type="button"
                        class="uc-btn uc-btn--ghost uc-btn--icon uc-btn--xs"
                        title="Delete"
                        aria-label="Delete"
                        on:click=move |_| deleting.set(Some(target.clone()))
                    >
                        <Trash2/>
                    </button>
                }
            })
        };
        view! {
            <tr>
                <td class="uc-strong">{label}</td>
                <td class="uc-muted uc-nowrap uc-right">{size}</td>
                <td class="uc-muted uc-nowrap">{modified}</td>
                <td class="uc-right uc-nowrap">
                    <span class="uc-files__actions">{download} {delete}</span>
                </td>
            </tr>
        }
    };

    let body = move || {
        loaded_list(entries.get(), "This folder is empty.", |list| {
            view! {
                <div class="uc-table-wrap">
                    <table class="uc-table">
                        <thead>
                            <tr>
                                <th>"Name"</th>
                                <th class="uc-right">"Size"</th>
                                <th>"Last modified"</th>
                                <th></th>
                            </tr>
                        </thead>
                        <tbody>{list.into_iter().map(row).collect_view()}</tbody>
                    </table>
                </div>
            }
            .into_any()
        })
    };
    let load_more = move || {
        next.get().map(|token| {
            view! {
                <button
                    type="button"
                    class="uc-btn uc-btn--outline uc-btn--sm uc-files__more"
                    disabled=move || more.get()
                    on:click=move |_| load(Some(token.clone()))
                >
                    {move || if more.get() { "Loading…" } else { "Load more" }}
                </button>
            }
        })
    };

    view! {
        <Card title="Files" actions>
            <nav class="uc-files__path" aria-label="Folder">{crumbs}</nav>
            {body}
            {load_more}
        </Card>
        <UploadModal open=upload_open folder then=move || load(None)/>
        <NewFolderModal open=folder_open folder then=move |name: String| dir.update(|d| d.push(name))/>
        <DeleteEntryModal entry=deleting then=move || load(None)/>
    }
}

/// "Upload files": files from this computer into the folder, replacing
/// files of the same name only when asked to.
#[component]
fn UploadModal(
    open: RwSignal<bool>,
    #[prop(into)] folder: Signal<String>,
    then: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let input = NodeRef::<html::Input>::new();
    let replace = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let close = move || {
        open.set(false);
        replace.set(false);
    };
    let on_ok = move || {
        let files: Vec<web_sys::File> = input
            .get_untracked()
            .and_then(|input| input.files())
            .map(|list| (0..list.length()).filter_map(|i| list.get(i)).collect())
            .unwrap_or_default();
        if files.is_empty() {
            notify.error("Choose the files to upload");
            return;
        }
        let folder = folder.get_untracked();
        let overwrite = replace.get_untracked();
        busy.set(true);
        spawn_local(async move {
            let total = files.len();
            let mut uploaded = 0;
            for file in files {
                let name = file.name();
                let contents = match JsFuture::from(file.array_buffer()).await {
                    Ok(buffer) => js_sys::Uint8Array::new(&buffer).to_vec(),
                    Err(_) => {
                        notify.error(format!("Cannot read {name}"));
                        break;
                    }
                };
                let result = api::client()
                    .upload_file(format!("{folder}/{name}"), contents)
                    .with_overwrite(overwrite)
                    .await;
                match result {
                    Ok(()) => uploaded += 1,
                    Err(e) => {
                        notify.failed(&e, &format!("Failed to upload {name}"));
                        break;
                    }
                }
            }
            if uploaded > 0 {
                let files = if uploaded == 1 { "file" } else { "files" };
                notify.success(format!("Uploaded {uploaded} {files}"));
                then();
            }
            // A failure keeps the dialog, so that it can be sent again.
            if busy.try_set(false).is_none() && uploaded == total {
                close();
            }
        });
    };
    view! {
        <Modal
            open
            title="Upload files"
            ok_text="Upload"
            busy_text="Uploading…"
            busy
            on_cancel=move || close()
            on_ok
        >
            <div class="uc-field">
                <label class="uc-label" for="uc-upload-files">"Files"</label>
                <input id="uc-upload-files" class="uc-input" type="file" multiple node_ref=input/>
            </div>
            <label class="uc-checkbox">
                <input
                    type="checkbox"
                    prop:checked=move || replace.get()
                    on:change=move |e| replace.set(event_target_checked(&e))
                />
                <span>"Replace files that are already there"</span>
            </label>
        </Modal>
    }
}

/// "New folder": a folder in the folder, which then shows.
#[component]
fn NewFolderModal(
    open: RwSignal<bool>,
    #[prop(into)] folder: Signal<String>,
    then: impl Fn(String) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let close = move || {
        open.set(false);
        name.set(String::new());
    };
    let on_ok = move || {
        let new_name = name.get_untracked().trim().to_string();
        if new_name.is_empty() || matches!(new_name.as_str(), "." | "..") || new_name.contains('/')
        {
            notify.error("Enter a folder name without /");
            return;
        }
        let path = format!("{}/{new_name}", folder.get_untracked());
        busy.set(true);
        spawn_local(async move {
            let result = api::client().create_directory(path).await;
            if busy.try_set(false).is_some() {
                return;
            }
            match result {
                Ok(()) => {
                    close();
                    then(new_name);
                }
                Err(e) => notify.failed(&e, "Failed to create the folder"),
            }
        });
    };
    view! {
        <Modal
            open
            title="New folder"
            ok_text="Create"
            busy
            on_cancel=move || close()
            on_ok
        >
            <TextField id="uc-folder-name" label="Name" value=name required=true/>
        </Modal>
    }
}

/// "Delete file?" or "Delete folder?". A folder that holds something gets
/// the box that deletes what it holds too, as the delete of a catalog has.
#[component]
fn DeleteEntryModal(
    entry: RwSignal<Option<DirectoryEntry>>,
    then: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let busy = RwSignal::new(false);
    // What the folder holds, counted on the first page of its entries:
    // `None` for a file.
    let held = RwSignal::new(None::<Load<Option<String>>>);
    let forced = RwSignal::new(false);
    let notify = use_notify();
    let noun = move || match entry.with(|e| e.as_ref().is_some_and(|e| e.is_directory)) {
        true => "folder",
        false => "file",
    };
    Effect::new(move |_| {
        let Some(target) = entry.get() else {
            return;
        };
        forced.set(false);
        if !target.is_directory {
            held.set(None);
            return;
        }
        held.set(Some(Load::Loading));
        spawn_local(async move {
            let page = api::client()
                .list_directory_contents(target.path.clone())
                .with_page_size(1000)
                .await;
            let counted = match page {
                Ok(page) if page.contents.is_empty() => Load::Ready(None),
                Ok(page) => {
                    Load::Ready(Some(holds(&page.contents, page.next_page_token.is_some())))
                }
                Err(e) if e.status() == Some(404) => Load::Ready(None),
                Err(e) => Load::Failed(api::failure(&e, "Failed to list the folder")),
            };
            // Only for the folder that the dialog still shows.
            let shown = entry.try_get_untracked().flatten();
            if shown.is_some_and(|e| e.path == target.path) {
                let _ = held.try_set(Some(counted));
            }
        });
    });
    let blocked = Signal::derive(move || match held.get() {
        None | Some(Load::Ready(None)) => false,
        Some(Load::Loading) => true,
        Some(Load::Ready(Some(_)) | Load::Failed(_)) => !forced.get(),
    });
    let on_ok = move || {
        let Some(target) = entry.get_untracked() else {
            return;
        };
        let recursive = forced.get_untracked();
        busy.set(true);
        spawn_local(async move {
            let client = api::client();
            let result = match target.is_directory {
                true => {
                    client
                        .delete_directory(target.path.clone())
                        .with_recursive(recursive)
                        .await
                }
                false => client.delete_file(target.path.clone()).await,
            };
            if busy.try_set(false).is_some() {
                return;
            }
            match result {
                Ok(()) => {
                    notify.success(format!("Deleted {}", target.name));
                    entry.set(None);
                    then();
                }
                Err(e) => notify.failed(&e, &format!("Failed to delete {}", target.name)),
            }
        });
    };
    view! {
        <Modal
            open=Signal::derive(move || entry.with(Option::is_some))
            title=move || format!("Delete {}?", noun())
            ok_text="Delete"
            busy_text="Deleting…"
            danger=true
            busy
            disabled=blocked
            on_cancel=move || entry.set(None)
            on_ok
        >
            <p>
                "Delete "
                <strong>{move || entry.with(|e| e.as_ref().map(|e| e.name.clone()))}</strong>
                "? This cannot be undone."
            </p>
            {move || {
                held.with(Option::is_some)
                    .then(|| {
                        view! {
                            <HoldsChoice
                                what="the folder"
                                held
                                label="Also delete the files and folders in it"
                                forced
                            />
                        }
                    })
            }}
        </Modal>
    }
}

/// What the entries of a folder's first page are, such as `2 folders and
/// 1 file`; `more` when there are more pages.
fn holds(entries: &[DirectoryEntry], more: bool) -> String {
    let count = |n: usize, one: &str, many: &str| match n {
        1 => format!("1 {one}"),
        n => format!("{n} {many}"),
    };
    if more {
        return format!(
            "more than {}",
            count(entries.len(), "file or folder", "files and folders")
        );
    }
    let folders = entries.iter().filter(|e| e.is_directory).count();
    let files = entries.len() - folders;
    match (folders, files) {
        (0, files) => count(files, "file", "files"),
        (folders, 0) => count(folders, "folder", "folders"),
        (folders, files) => format!(
            "{} and {}",
            count(folders, "folder", "folders"),
            count(files, "file", "files")
        ),
    }
}

/// `root` and the folders under it, as one path.
fn join(root: &str, parts: &[String]) -> String {
    let mut path = root.trim_end_matches('/').to_string();
    for part in parts {
        path.push('/');
        path.push_str(part);
    }
    path
}

#[cfg(test)]
mod tests {
    use uc_client::models::files::v1::DirectoryEntry;

    use super::{PATH, holds, join};

    #[test]
    fn what_a_folder_holds() {
        let entry = |is_directory| DirectoryEntry {
            is_directory,
            ..Default::default()
        };
        assert_eq!(holds(&[entry(false)], false), "1 file");
        assert_eq!(holds(&[entry(true), entry(true)], false), "2 folders");
        assert_eq!(
            holds(&[entry(true), entry(false), entry(false)], false),
            "1 folder and 2 files"
        );
        assert_eq!(
            holds(&[entry(true), entry(false)], true),
            "more than 2 files and folders"
        );
    }

    #[test]
    fn paths() {
        assert_eq!(join("/Volumes/c/s/v", &[]), "/Volumes/c/s/v");
        let parts = ["a".to_string(), "b c".to_string()];
        assert_eq!(join("/Volumes/c/s/v/", &parts), "/Volumes/c/s/v/a/b c");
        let encoded = percent_encoding::utf8_percent_encode("/Volumes/c/s/v/a b#1%~.txt", PATH);
        assert_eq!(encoded.to_string(), "/Volumes/c/s/v/a%20b%231%25~.txt");
    }
}
