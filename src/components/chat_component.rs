use leptos::ev::KeyboardEvent;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::view;
use leptos::web_sys;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map};
use serde::Serialize;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::JsCast;

use super::message_component::Message;
use super::models;
use models::{ChatMessage, Friend, GroupStore, MessageStore, User, XmppSendMessageArgs};

// Import the app's invoke function
use crate::app::invoke;

#[derive(Serialize)]
struct XmppSendGroupMessageArgs {
    #[serde(rename = "roomJid")]
    room_jid: String,
    body: String,
}

#[derive(Serialize)]
struct XmppFetchGroupHistoryArgs {
    #[serde(rename = "roomJid")]
    room_jid: String,
}

#[derive(Serialize)]
struct XmppSendNudgeArgs {
    #[serde(rename = "toJid")]
    to_jid: String,
}

#[derive(Serialize)]
struct XmppSetGroupSubjectArgs {
    #[serde(rename = "roomJid")]
    room_jid: String,
    subject: String,
}

#[derive(Serialize)]
struct XmppInviteToGroupArgs {
    #[serde(rename = "roomJid")]
    room_jid: String,
    #[serde(rename = "roomName")]
    room_name: String,
    invitees: Vec<String>,
}

#[derive(Serialize)]
struct XmppLeaveGroupArgs {
    #[serde(rename = "roomJid")]
    room_jid: String,
}

#[derive(Serialize)]
struct XmppRemoveGroupMemberArgs {
    #[serde(rename = "roomJid")]
    room_jid: String,
    #[serde(rename = "memberJid")]
    member_jid: String,
}

#[component]
pub fn Chat() -> impl IntoView {
    let (msg, set_msg) = signal(String::new());
    let update_msg = move |ev| set_msg.set(event_target_value(&ev));
    let (nudge_cooldown, set_nudge_cooldown) = signal(false);
    let (show_rename_modal, set_show_rename_modal) = signal(false);
    let (show_invite_modal, set_show_invite_modal) = signal(false);
    let (rename_value, set_rename_value) = signal(String::new());
    let (selected_invitees, set_selected_invitees) = signal(Vec::<String>::new());
    let (group_action_status, set_group_action_status) = signal(Option::<String>::None);

    let set_open_chats = use_context::<WriteSignal<Vec<String>>>();
    let friends =
        use_context::<ReadSignal<(Vec<Friend>, Vec<Friend>)>>().expect("No friends context");
    let user = use_context::<ReadSignal<User>>().expect("No user context");
    // Global message store provided by App
    let messages = use_context::<RwSignal<MessageStore>>().expect("No messages context");
    let groups = use_context::<RwSignal<GroupStore>>().expect("No groups context");

    let navigate = use_navigate();

    // Read the bare JID from the route parameter (e.g. /chat/alice@localhost)
    let jid_param = use_params_map()
        .get_untracked()
        .get("id")
        .unwrap_or_default();
    let is_group = jid_param.contains("@conference.");
    let can_manage_group = {
        let jid = jid_param.clone();
        move || {
            if !jid.contains("@conference.") {
                return false;
            }
            let own_jid = user
                .get_untracked()
                .email
                .split('/')
                .next()
                .unwrap_or("")
                .to_string();
            groups
                .get()
                .get(&jid)
                .map(|group| {
                    if group.owner == own_jid {
                        return true;
                    }
                    group.members.iter().any(|member| {
                        member.jid == own_jid && (member.role == "owner" || member.role == "admin")
                    })
                })
                .unwrap_or(false)
        }
    };
    let can_manage_group_signal = Memo::new(move |_| can_manage_group());
    let (group_route, set_group_route) = signal(Option::<String>::None);
    Effect::new({
        let navigate = navigate.clone();
        move |_| {
            if let Some(route) = group_route.get() {
                navigate(&route, Default::default());
                set_group_route.set(None);
            }
        }
    });

    // Look up the friend by JID. Returns a clone each call — two separate closures used in view.
    let friend_info_name = {
        let jid = jid_param.clone();
        move || {
            if jid.contains("@conference.") {
                return groups
                    .get()
                    .get(&jid)
                    .map(|group| group.name.clone())
                    .unwrap_or_else(|| jid.clone());
            }
            let (online, offline) = friends.get();
            online
                .into_iter()
                .chain(offline)
                .find(|f| f.email == jid)
                .map(|f| f.name)
                .unwrap_or_else(|| jid.clone())
        }
    };
    let friend_info_status = {
        let jid = jid_param.clone();
        move || {
            if jid.contains("@conference.") {
                return groups
                    .get()
                    .get(&jid)
                    .map(|group| {
                        let count = group.members.len();
                        if count == 0 {
                            "Group conversation".to_string()
                        } else {
                            format!("{count} member{}", if count == 1 { "" } else { "s" })
                        }
                    })
                    .unwrap_or_else(|| "Group conversation".to_string());
            }
            let (online, offline) = friends.get();
            online
                .into_iter()
                .chain(offline)
                .find(|f| f.email == jid)
                .map(|f| format!("{} <{}>", f.flavour_text, f.email))
                .unwrap_or_default()
        }
    };

    // Close this chat and return to main page
    let close_chat = {
        let jid = jid_param.clone();
        move |ev: leptos::ev::MouseEvent| {
            ev.prevent_default();
            ev.stop_propagation();
            if let Some(set_open_chats) = set_open_chats {
                let jid = jid.clone();
                set_open_chats.update(|chats| chats.retain(|j| j != &jid));
            }
            set_group_route.set(Some("/main".to_string()));
        }
    };

    let submit_on_enter = move |ev: KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            if let Some(form) = document().get_element_by_id("message-form") {
                if let Some(element) = form.dyn_ref::<web_sys::HtmlFormElement>() {
                    let _ = element.request_submit();
                }
            }
        }
    };

    let send_msg = {
        let jid = jid_param.clone();
        move |ev: SubmitEvent| {
            ev.prevent_default();
            let jid = jid.clone();
            let messages = messages;
            spawn_local(async move {
                let body = msg.get_untracked();
                if body.trim().is_empty() {
                    return;
                }
                let body = body.trim().to_string();

                // Optimistically append to global store with pending=true (not yet confirmed by server)
                let self_jid = user.get_untracked().email;
                let optimistic_msg = ChatMessage {
                    id: String::new(),
                    from_jid: self_jid,
                    body: body.clone(),
                    timestamp: String::new(),
                    is_self: true,
                    pending: true,
                    author_nickname: None,
                };
                let mut pushed_idx: usize = 0;
                messages.update(|store| {
                    let conv = store.entry(jid.clone()).or_default();
                    conv.push(optimistic_msg);
                    if conv.len() > 200 {
                        conv.drain(0..conv.len() - 200);
                    }
                    pushed_idx = conv.len() - 1;
                });

                set_msg.set(String::new());
                if let Some(input) = document().get_element_by_id("message-input") {
                    if let Some(el) = input.dyn_ref::<web_sys::HtmlTextAreaElement>() {
                        el.set_value("");
                    }
                }

                // Send via XMPP
                let is_group = jid.contains("@conference.");
                let result = if is_group {
                    invoke(
                        "xmpp_send_group_message",
                        to_value(&XmppSendGroupMessageArgs {
                            room_jid: jid.clone(),
                            body: body.clone(),
                        })
                        .unwrap(),
                    )
                    .await
                } else {
                    let args = XmppSendMessageArgs {
                        to_jid: jid.clone(),
                        body: body.clone(),
                    };
                    invoke("xmpp_send_message", to_value(&args).unwrap()).await
                };
                match from_value::<serde_json::Value>(result) {
                    Ok(response) if response["success"].as_bool().unwrap_or(false) => {
                        // Confirm delivery — clear the pending flag
                        messages.update(|store| {
                            if let Some(conv) = store.get_mut(&jid) {
                                if let Some(msg) = conv.get_mut(pushed_idx) {
                                    msg.pending = false;
                                }
                            }
                        });
                        leptos::logging::log!("Message sent to {}", jid);
                    }
                    Ok(response) => {
                        leptos::logging::warn!("xmpp_send_message error: {:?}", response["error"]);
                    }
                    Err(e) => {
                        leptos::logging::warn!("xmpp_send_message parse error: {:?}", e);
                    }
                }
            });
        }
    };

    // Register this JID in open_chats so the main page tab tracks it
    if let Some(set_open_chats) = set_open_chats {
        let jid = jid_param.clone();
        set_open_chats.update(|chats| {
            if !chats.contains(&jid) {
                chats.push(jid);
            }
        });
        if jid_param.contains("@conference.") {
            let jid = jid_param.clone();
            groups.update(|store| {
                if let Some(group) = store.get_mut(&jid) {
                    group.unread = 0;
                }
            });
        }
    }

    if is_group {
        let room_jid = jid_param.clone();
        spawn_local(async move {
            let _ = invoke(
                "xmpp_fetch_group_history",
                to_value(&XmppFetchGroupHistoryArgs { room_jid }).unwrap(),
            )
            .await;
        });
    }

    // Derive the displayed message list reactively from the global store
    let displayed_messages = {
        let jid = jid_param.clone();
        move || -> Vec<ChatMessage> { messages.get().get(&jid).cloned().unwrap_or_default() }
    };
    // Clone for the author lookup inside the message rendering closure
    let jid_for_msgs = jid_param.clone();
    let jid_action = jid_param.clone();
    let jid_action_rename = jid_action.clone();
    let jid_action_invite = jid_action.clone();

    let mute_group = {
        let jid = jid_action.clone();
        Callback::new(move |_: ()| {
            groups.update(|store| {
                if let Some(group) = store.get_mut(&jid) {
                    group.muted = !group.muted;
                }
            });
        })
    };

    let leave_group = {
        let jid = jid_action.clone();
        let set_open_chats = set_open_chats;
        Callback::new(move |_: ()| {
            let jid = jid.clone();
            spawn_local(async move {
                let _ = invoke(
                    "xmpp_leave_group",
                    to_value(&XmppLeaveGroupArgs {
                        room_jid: jid.clone(),
                    })
                    .unwrap(),
                )
                .await;
                if let Some(set_open_chats) = set_open_chats {
                    set_open_chats.update(|chats| chats.retain(|chat| chat != &jid));
                }
                groups.update(|store| {
                    store.remove(&jid);
                });
            });
            set_group_route.set(Some("/main".to_string()));
        })
    };

    let remove_member = {
        let jid = jid_action.clone();
        Callback::new(move |_: ()| {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(member_jid)) = window.prompt_with_message("JID do membro a remover")
                {
                    let member_jid = member_jid.trim().to_string();
                    if member_jid.is_empty() {
                        return;
                    }
                    let jid = jid.clone();
                    spawn_local(async move {
                        let _ = invoke(
                            "xmpp_remove_group_member",
                            to_value(&XmppRemoveGroupMemberArgs {
                                room_jid: jid,
                                member_jid,
                            })
                            .unwrap(),
                        )
                        .await;
                    });
                }
            }
        })
    };

    view! {
        <main class="container">
            <div class="chat_container">
                <div class="chat_receiver-bar">
                    <div class="chat-header-left">
                        <A href="/main">
                            <button class="back-button chat-icon-button" title="Back to Main" aria-label="Back to Main">
                                <img src="/assets/nto_icons_png/back.png" alt="" class="chat-header-icon" />
                            </button>
                        </A>
                    </div>

                    <div class="chat-header-center">
                        <span class="chat_receiver">
                            <img src="/assets/nto-connected-brand-assets/nto-app-icon-32.png" alt="" class="chat-contact-icon" />
                            {move || format!(" {}", friend_info_name())}
                        </span>
                        <span class="chat_receiver-status-message">
                            {move || friend_info_status()}
                        </span>
                    </div>

                    <div class="chat-header-right">
                        <Show
                            when=move || is_group
                            fallback=move || view! { <div></div> }
                        >
                            <div class="group-header-actions">
                                <Show
                                    when=move || can_manage_group_signal.get()
                                    fallback=move || view! { <span></span> }
                                >
                                    <button
                                        class="group-action-button"
                                        title="Rename group"
                                        on:click=move |_| {
                                            set_rename_value.set(String::new());
                                            set_show_rename_modal.set(true);
                                        }
                                    >
                                        "Name"
                                    </button>
                                    <button
                                        class="group-action-button"
                                        title="Invite friends"
                                        on:click=move |_| {
                                            set_selected_invitees.set(Vec::new());
                                            set_show_invite_modal.set(true);
                                        }
                                    >
                                        "Invite"
                                    </button>
                                    <button
                                        class="group-action-button"
                                        title="Remove member"
                                        on:click=move |_| remove_member.run(())
                                    >
                                        "Remove"
                                    </button>
                                </Show>
                                <button
                                    class="group-mute-button"
                                    title="Mute group"
                                    on:click=move |_| mute_group.run(())
                                >
                                    "Mute"
                                </button>
                                <button
                                    class="group-leave-button"
                                    title="Leave group"
                                    on:click=move |_| leave_group.run(())
                                >
                                    "Leave"
                                </button>
                            </div>
                        </Show>
                        <button class="close-button chat-icon-button" on:click=close_chat title="Close Chat" aria-label="Close Chat">
                            <img src="/assets/nto_icons_png/close.png" alt="" class="chat-header-icon" />
                        </button>
                    </div>
                </div>
                <div class="chat_top-bar chat_icon-bar main_bordered">
                    <div class="chat_config-btn" title="Settings"><img src="/assets/nto_icons_png/settings.png" alt="Settings" class="chat-toolbar-icon" /></div>
                    <div class="chat_invite-btn" title="Contact card"><img src="/assets/nto_icons_png/contact-card.png" alt="Contact card" class="chat-toolbar-icon" /></div>
                    <div class="chat_files-btn" title="Send file"><img src="/assets/nto_icons_png/files.png" alt="Send file" class="chat-toolbar-icon" /></div>
                    <div class="chat_webcam-btn" title="Video call"><img src="/assets/nto_icons_png/video-call.png" alt="Video call" class="chat-toolbar-icon" /></div>
                    <div class="chat_voice-btn" title="Voice call"><img src="/assets/nto_icons_png/voice-call.png" alt="Voice call" class="chat-toolbar-icon" /></div>
                    <div class="chat_block-btn" title="Block"><img src="/assets/nto_icons_png/block.png" alt="Block" class="chat-toolbar-icon" /></div>
                </div>
                <div class="flex-row chat-and-avatars">
                    <div class="left-column">
                        <div class="chat_window">
                            <div class="chat_message-list main_bordered">
                                {move || {
                                    let user_name = user.get().name;
                                    let (online, offline) = friends.get();
                                    let friend_name = online
                                        .into_iter()
                                        .chain(offline)
                                        .find(|f| f.email == jid_for_msgs)
                                        .map(|f| f.name)
                                        .unwrap_or_else(|| {
                                            jid_for_msgs
                                                .split('@')
                                                .next()
                                                .unwrap_or(&jid_for_msgs)
                                                .to_string()
                                        });
                                    displayed_messages()
                                        .into_iter()
                                        .map(|m| {
                                            let author = if m.is_self {
                                                user_name.clone()
                                            } else {
                                                let nickname = m.author_nickname
                                                    .clone()
                                                    .filter(|nickname| !nickname.is_empty())
                                                    .unwrap_or_default();
                                                let mut resolved = None;
                                                if is_group {
                                                    if let Some(group) = groups.get().get(&jid_for_msgs) {
                                                        if let Some(member) = group.members.iter().find(|member| {
                                                            member.nickname == nickname
                                                        }) {
                                                            if !member.jid.is_empty() {
                                                                let (online, offline) = friends.get();
                                                                resolved = online
                                                                    .iter()
                                                                    .chain(offline.iter())
                                                                    .find(|friend| friend.email == member.jid)
                                                                    .map(|friend| friend.name.clone());
                                                            }
                                                        }
                                                    }
                                                }
                                                resolved.unwrap_or_else(|| {
                                                    match nickname.as_str() {
                                                        "testuser" => "Pedro".to_string(),
                                                        "kampelo" => "Kampelo".to_string(),
                                                        "ivokds" => "Ivokds".to_string(),
                                                        "" => friend_name.clone(),
                                                        other => other.to_string(),
                                                    }
                                                })
                                            };
                                            view! {
                                                <Message author=author body=m.body pending=m.pending />
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                }}
                            </div>
                        </div>
                        <div class="chat_mid-bar chat_icon-bar main_bordered">
                            <div class="chat_font-btn" title="Text format"><img src="/assets/nto_icons_png/text-format.png" alt="Text format" class="chat-toolbar-icon" /></div>
                            <div class="chat_emote-btn" title="Emoticons"><img src="/assets/nto_icons_png/emoji.png" alt="Emoticons" class="chat-toolbar-icon" /></div>
                            <div class="chat_audio-btn" title="Wink"><img src="/assets/nto_icons_png/wink.png" alt="Wink" class="chat-toolbar-icon" /></div>
                            <div class="chat_image-btn" title="Send image"><img src="/assets/nto_icons_png/send-image.png" alt="Send image" class="chat-toolbar-icon" /></div>
                            <Show
                                when=move || !is_group
                                fallback=move || view! { <div class="chat_nudge-btn chat_nudge-btn--disabled"></div> }
                            >
                                <button
                                    class="chat_nudge-btn"
                                    title="Nudge"
                                    aria-label="Send Nudge"
                                    prop:disabled=move || nudge_cooldown.get()
                                    on:click={
                                        let jid = jid_param.clone();
                                        move |ev: leptos::ev::MouseEvent| {
                                            let jid = jid.clone();
                                            set_nudge_cooldown.set(true);
                                            if let Some(target) =
                                                ev.target().and_then(|target| target.dyn_ref::<web_sys::HtmlButtonElement>().cloned())
                                            {
                                                let _ = target.class_list().add_1("nto-nudge-success");
                                                let target_for_cleanup = target.clone();
                                                spawn_local(async move {
                                                    gloo_timers::future::TimeoutFuture::new(850).await;
                                                    let _ = target_for_cleanup
                                                        .class_list()
                                                        .remove_1("nto-nudge-success");
                                                });
                                            }
                                            spawn_local(async move {
                                                let _ = invoke(
                                                    "xmpp_send_nudge",
                                                    to_value(&XmppSendNudgeArgs { to_jid: jid })
                                                        .unwrap(),
                                                )
                                                .await;
                                            });
                                            spawn_local(async move {
                                                gloo_timers::future::TimeoutFuture::new(1500).await;
                                                set_nudge_cooldown.set(false);
                                            });
                                        }
                                    }
                                >
                                    <img src="/assets/nto_icons_png/nudge.png" alt="Nudge" class="chat-toolbar-icon" />
                                </button>
                            </Show>
                        </div>
                        <div class="chat_message-input">
                            <form id="message-form" class="chat_message-form" on:submit=send_msg>
                                <textarea
                                    class="main_bordered"
                                    id="message-input"
                                    placeholder="Enter a message..."
                                    on:input=update_msg
                                    on:keydown=submit_on_enter
                                ></textarea>
                                <button type="submit">"Send"</button>
                            </form>
                        </div>
                    </div>
                    <div class="flex-col right-column">
                        <div class="flex-col top-user user-block">
                            <div class="top-avatar avatar"></div>
                            <div class="below-avatar">
                                <div class="webcam-icon" title="Video call"><img src="/assets/nto_icons_png/video-call.png" alt="Video call" class="avatar-action-icon" /></div>
                                <div class="options-arrow" title="More options"><img src="/assets/nto_icons_png/dropdown.png" alt="More options" class="avatar-action-icon" /></div>
                            </div>
                        </div>
                        <div class="flex-col bottom-user user-block">
                            <div class="bottom-avatar avatar"></div>
                            <div class="below-avatar">
                                <div class="webcam-icon" title="Video call"><img src="/assets/nto_icons_png/video-call.png" alt="Video call" class="avatar-action-icon" /></div>
                                <div class="options-arrow" title="More options"><img src="/assets/nto_icons_png/dropdown.png" alt="More options" class="avatar-action-icon" /></div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            <Show when=move || show_rename_modal.get()>
                <div class="group-modal-backdrop">
                    <div class="group-modal group-rename-modal" role="dialog" aria-modal="true">
                        <div class="group-modal-header">
                            <div>
                                <span class="group-modal-kicker">"Group settings"</span>
                                <h2>"Rename group"</h2>
                            </div>
                            <button
                                type="button"
                                class="group-modal-close"
                                aria-label="Close"
                                on:click=move |_| set_show_rename_modal.set(false)
                            >
                                "×"
                            </button>
                        </div>
                        <label for="group-name-input">"Group name"</label>
                        <input
                            id="group-name-input"
                            type="text"
                            maxlength="80"
                            prop:value=move || rename_value.get()
                            on:input=move |ev| set_rename_value.set(event_target_value(&ev))
                        />
                        <div class="group-modal-actions">
                            <button
                                type="button"
                                class="group-modal-cancel"
                                on:click=move |_| set_show_rename_modal.set(false)
                            >
                                "Cancel"
                            </button>
                            <button
                                type="button"
                                class="group-modal-confirm"
                                on:click={
                                    let jid = jid_action_rename.clone();
                                    move |_| {
                                        let subject = rename_value.get_untracked().trim().to_string();
                                        if subject.is_empty() {
                                            set_group_action_status.set(Some("Group name is required".to_string()));
                                            return;
                                        }
                                        let jid = jid.clone();
                                        spawn_local(async move {
                                            let _ = invoke(
                                                "xmpp_set_group_subject",
                                                to_value(&XmppSetGroupSubjectArgs {
                                                    room_jid: jid.clone(),
                                                    subject: subject.clone(),
                                                })
                                                .unwrap(),
                                            )
                                            .await;
                                            groups.update(|store| {
                                                if let Some(group) = store.get_mut(&jid) {
                                                    group.name = subject.clone();
                                                    group.subject = Some(subject);
                                                }
                                            });
                                        });
                                        set_show_rename_modal.set(false);
                                        set_group_action_status.set(None);
                                    }
                                }
                            >
                                "Save"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
            <Show when=move || show_invite_modal.get()>
                <div class="group-modal-backdrop">
                    <div class="group-modal group-invite-modal" role="dialog" aria-modal="true">
                        <div class="group-modal-header">
                            <div>
                                <span class="group-modal-kicker">"Group members"</span>
                                <h2>"Invite friends"</h2>
                            </div>
                            <button
                                type="button"
                                class="group-modal-close"
                                aria-label="Close"
                                on:click=move |_| set_show_invite_modal.set(false)
                            >
                                "×"
                            </button>
                        </div>
                        <div class="group-member-heading">
                            "Online friends"
                            <span>{move || format!(" ({})", selected_invitees.get().len())}</span>
                        </div>
                        <div class="group-member-list">
                            {move || {
                                let (online, _) = friends.get();
                                online
                                    .into_iter()
                                    .map(|friend| {
                                        let jid = friend.email.clone();
                                        let jid_toggle = jid.clone();
                                        let selected = selected_invitees.get().contains(&jid);
                                        view! {
                                            <button
                                                type="button"
                                                class="group-member-option"
                                                class:selected=selected
                                                on:click=move |_| {
                                                    set_selected_invitees.update(|selected| {
                                                        if selected.contains(&jid_toggle) {
                                                            selected.retain(|item| item != &jid_toggle);
                                                        } else {
                                                            selected.push(jid_toggle.clone());
                                                        }
                                                    });
                                                }
                                            >
                                                <span class="group-member-avatar">"👤"</span>
                                                <span class="group-member-name">{friend.name.clone()}</span>
                                                <span class="group-member-check">
                                                    {if selected { "✓" } else { "" }}
                                                </span>
                                            </button>
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            }}
                        </div>
                        {move || group_action_status.get().map(|status| view! {
                            <div class="group-create-status">{status}</div>
                        })}
                        <div class="group-modal-actions">
                            <button
                                type="button"
                                class="group-modal-cancel"
                                on:click=move |_| set_show_invite_modal.set(false)
                            >
                                "Cancel"
                            </button>
                            <button
                                type="button"
                                class="group-modal-confirm"
                                on:click={
                                    let jid = jid_action_invite.clone();
                                    move |_| {
                                        let invitees = selected_invitees.get_untracked();
                                        if invitees.is_empty() {
                                            set_group_action_status.set(Some("Select at least one friend".to_string()));
                                            return;
                                        }
                                        let room_name = groups
                                            .get()
                                            .get(&jid)
                                            .map(|group| group.name.clone())
                                            .unwrap_or_else(|| "Group".to_string());
                                        let room_jid = jid.clone();
                                        spawn_local(async move {
                                            let _ = invoke(
                                                "xmpp_invite_to_group",
                                                to_value(&XmppInviteToGroupArgs {
                                                    room_jid,
                                                    room_name,
                                                    invitees,
                                                })
                                                .unwrap(),
                                            )
                                            .await;
                                        });
                                        set_show_invite_modal.set(false);
                                        set_selected_invitees.set(Vec::new());
                                        set_group_action_status.set(None);
                                    }
                                }
                            >
                                "Send invitations"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </main>
    }
}
