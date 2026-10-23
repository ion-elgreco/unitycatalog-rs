//! A group's page, for admins: its members, with a × that removes each, a
//! form that adds a user, and "Delete".

use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_api::{ScimGroup, ScimGroupMember, ScimUser};

use crate::api;
use crate::auth::use_auth;
use crate::components::icons::X;
use crate::components::{
    AdminOnly, Card, DeleteButton, DeleteModal, Empty, EntityHeader, Loaded, use_notify,
};
use crate::load::{Key, Load, use_cache, use_load};
use crate::util::{Section, or_dash, route_params};

#[component]
pub fn GroupDetails() -> impl IntoView {
    let params = route_params(["name"]);
    move || {
        let [name] = params.get();
        view! {
            <AdminOnly title="Groups">
                <GroupPage name=name.clone()/>
            </AdminOnly>
        }
    }
}

#[component]
fn GroupPage(name: String) -> impl IntoView {
    let key = StoredValue::new(name.clone());
    let info = use_load(Key::Group(name.clone()), move || {
        let name = key.get_value();
        async move { api::group(&name).await }
    });
    let users = use_load(Key::Users, api::users);
    let auth = use_auth();
    let cache = use_cache();
    let notify = use_notify();
    let delete_open = RwSignal::new(false);
    let busy = RwSignal::new(false);
    // The user to add, by id.
    let adding = RwSignal::new(String::new());
    let admin = Memo::new(move |_| auth.session.with(|s| s.is_admin()));
    let members = Signal::derive(move || {
        info.with(|g| {
            g.ready()
                .and_then(|g| g.members.clone())
                .unwrap_or_default()
        })
    });
    let refresh = Callback::new(move |_| {
        cache.changed(Key::Group(key.get_value()));
        cache.changed(Key::Users);
    });
    // Save the group with `members`; the page shows the server's answer.
    let set_members = move |members: Vec<ScimGroupMember>| {
        let Some(group) = info.with_untracked(|g| g.ready().cloned()) else {
            return;
        };
        let Some(id) = group.id.clone() else {
            notify.error("The group has no id.");
            return;
        };
        let name = key.get_value();
        let group = ScimGroup {
            members: Some(members),
            ..group
        };
        busy.set(true);
        spawn_local(async move {
            match api::client().update_group(&id, &group).await {
                Ok(updated) => {
                    cache.set(Key::Group(name), updated);
                    cache.changed(Key::Groups);
                    notify.success("Members updated");
                }
                Err(e) => notify.failed(&e, "Failed to update the group"),
            }
            busy.set(false);
        });
    };
    let remove = move |id: String| {
        let rest = members
            .get_untracked()
            .into_iter()
            .filter(|m| m.value != id)
            .collect();
        set_members(rest);
    };
    let add = move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        let id = adding.get_untracked();
        if id.is_empty() {
            return;
        }
        let mut all = members.get_untracked();
        if all.iter().any(|m| m.value == id) {
            return;
        }
        all.push(ScimGroupMember {
            value: id,
            display: None,
            r#type: Some("User".into()),
        });
        adding.set(String::new());
        set_members(all);
    };
    // The user with `id`, from the list of users.
    let user_of = move |id: &str| {
        users.with(|users| {
            users
                .ready()
                .and_then(|list| list.iter().find(|u| u.id.as_deref() == Some(id)).cloned())
        })
    };
    // The users who are not members yet, by name.
    let candidates = Signal::derive(move || {
        let members = members.get();
        let mut list: Vec<ScimUser> = users.with(|users| {
            users
                .ready()
                .map(|list| {
                    list.iter()
                        .filter(|u| {
                            u.id.as_ref()
                                .is_some_and(|id| !members.iter().any(|m| m.value == *id))
                        })
                        .cloned()
                        .collect()
                })
                .unwrap_or_default()
        });
        list.sort_by_key(|u| {
            u.display_name
                .clone()
                .unwrap_or_else(|| u.user_name.clone())
        });
        list
    });
    let title = move || format!("Members ({})", members.with(Vec::len));
    let rows = move || {
        let members = members.get();
        if members.is_empty() {
            let empty = Empty::new("No members.", "Pick a user below and click Add member.");
            return view! { <crate::components::EmptyState empty/> }.into_any();
        }
        view! {
            <div class="uc-table-wrap">
                <table class="uc-table">
                    <thead>
                        <tr>
                            <th>"Name"</th>
                            <th>"Email"</th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>
                        {members
                            .into_iter()
                            .map(|member| {
                                let user = user_of(&member.value);
                                let name = user
                                    .as_ref()
                                    .and_then(|u| u.display_name.clone())
                                    .or_else(|| member.display.clone());
                                let email = user
                                    .as_ref()
                                    .map(|u| u.user_name.clone())
                                    .unwrap_or_else(|| member.value.clone());
                                let label = format!("Remove {email} from the group");
                                let id = member.value.clone();
                                view! {
                                    <tr>
                                        <td class="uc-strong">{or_dash(name)}</td>
                                        <td>{email}</td>
                                        <td class="uc-right">
                                            <button
                                                type="button"
                                                class="uc-btn uc-btn--ghost uc-btn--icon uc-btn--xs"
                                                title=label.clone()
                                                aria-label=label
                                                disabled=move || busy.get()
                                                on:click=move |_| remove(id.clone())
                                            >
                                                <X/>
                                            </button>
                                        </td>
                                    </tr>
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        }
        .into_any()
    };
    let picker = move || match users.get() {
        Load::Loading => view! { <p class="uc-hint">"Loading the users…"</p> }.into_any(),
        Load::Failed(message) => view! { <p class="uc-error">{message}</p> }.into_any(),
        Load::Ready(_) => {
            let list = candidates.get();
            view! {
                <form class="uc-form-row" on:submit=add>
                    <div class="uc-field">
                        <label class="uc-label" for="uc-group-user">"User"</label>
                        <select
                            id="uc-group-user"
                            class="uc-select"
                            on:change=move |e| adding.set(event_target_value(&e))
                        >
                            <option value="" prop:selected=move || adding.with(String::is_empty)>
                                "Choose a user"
                            </option>
                            {list
                                .into_iter()
                                .filter_map(|u| Some((u.id?, u.display_name, u.user_name)))
                                .map(|(id, name, email)| {
                                    let text = match name {
                                        Some(name) if !name.is_empty() => format!("{name} ({email})"),
                                        _ => email,
                                    };
                                    let this = id.clone();
                                    view! {
                                        <option value=id prop:selected=move || adding.with(|v| *v == this)>
                                            {text}
                                        </option>
                                    }
                                })
                                .collect_view()}
                        </select>
                    </div>
                    <button
                        type="submit"
                        class="uc-btn"
                        disabled=move || busy.get() || adding.with(String::is_empty)
                    >
                        "Add member"
                    </button>
                </form>
            }
            .into_any()
        }
    };

    view! {
        <EntityHeader
            icon=Section::Groups.icon()
            name=name.clone()
            badges=vec!["Group".to_string()]
            on_refresh=refresh
            actions=view! { <DeleteButton open=delete_open when=admin/> }.into_any()
        />
        <div class="uc-page">
            <Loaded state=info>
                <Card title=title>
                    {rows}
                    {picker}
                </Card>
            </Loaded>
        </div>
        <DeleteModal
            open=delete_open
            noun="group"
            name=name.clone()
            delete=move |_force| {
                let id = info.with_untracked(|g| g.ready().and_then(|g| g.id.clone()));
                async move {
                    api::client().delete_group(&id.unwrap_or_default()).await
                }
            }
            key=Key::Group(name.clone())
            list=Key::Groups
            then=Section::Groups.href().to_string()
        />
    }
}
