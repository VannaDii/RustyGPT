use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::api::RustyGPTClient;
use crate::components::{
    StreamingDisplay, ThreadComposer, ThreadList, ThreadView, TypingIndicator,
};
use chrono::Utc;
use gloo_timers::callback::Timeout;
use serde_json::from_str;
use shared::models::{
    ConversationStreamEvent, MembershipChangeAction, MessageRole, MessageView,
    PostRootMessageRequest, PresenceStatus, ReplyMessageRequest, ThreadSummary, Timestamp,
};
use uuid::Uuid;
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{EventSource, MessageEvent};
use yew::{
    Callback, Html, Properties, UseStateHandle, function_component, html, use_effect_with,
    use_mut_ref, use_state,
};

#[derive(Clone, PartialEq, Eq)]
struct StreamingEntry {
    message_id: Uuid,
    root_id: Uuid,
    parent_id: Option<Uuid>,
    depth: i32,
    conversation_id: Uuid,
    content: String,
}

#[derive(Properties, PartialEq, Eq)]
pub struct ChatPageProps {
    #[prop_or(None)]
    pub conversation_id: Option<String>,
}

type ListenerRegistry = Rc<RefCell<Vec<Closure<dyn FnMut(MessageEvent)>>>>;

#[allow(
    clippy::type_complexity,
    clippy::too_many_arguments,
    clippy::too_many_lines
)] // Tracking: TODO web-chat-001
fn register_stream_listeners(
    event_source: &EventSource,
    conversation_id: Uuid,
    listeners: &ListenerRegistry,
    threads: &UseStateHandle<Vec<ThreadSummary>>,
    selected_thread: &UseStateHandle<Option<Uuid>>,
    messages: &UseStateHandle<Vec<MessageView>>,
    error: &UseStateHandle<Option<String>>,
    typing: &UseStateHandle<bool>,
    typing_timer: &Rc<RefCell<Option<Timeout>>>,
    streaming: &UseStateHandle<HashMap<Uuid, StreamingEntry>>,
    pending_activity: &UseStateHandle<HashSet<Uuid>>,
    online_users: &UseStateHandle<HashSet<Uuid>>,
    unread_counts: &UseStateHandle<HashMap<Uuid, i64>>,
) {
    // thread.new
    {
        let threads = threads.clone();
        let listener =
            Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(move |event: MessageEvent| {
                if let Some(data) = event.data().as_string()
                    && let Ok(ConversationStreamEvent::ThreadNew { payload }) = from_str(&data)
                {
                    threads.set({
                        let mut next = (*threads).clone();
                        next.retain(|item| item.root_id != payload.root_id);
                        next.push(payload.summary.clone());
                        next.sort_by(|a, b| b.last_activity_at.0.cmp(&a.last_activity_at.0));
                        next
                    });
                }
            }));
        event_source
            .add_event_listener_with_callback("thread.new", listener.as_ref().unchecked_ref())
            .expect("thread.new listener");
        listeners.borrow_mut().push(listener);
    }

    // thread.activity
    {
        let threads = threads.clone();
        let selected_thread = selected_thread.clone();
        let messages = messages.clone();
        let error = error.clone();
        let pending_activity = pending_activity.clone();
        let listener =
            Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(move |event: MessageEvent| {
                if let Some(data) = event.data().as_string()
                    && let Ok(ConversationStreamEvent::ThreadActivity { payload }) = from_str(&data)
                {
                    let skip_fetch = (*pending_activity).contains(&payload.root_id);

                    threads.set({
                        let mut next = (*threads).clone();
                        if let Some(thread) = next.iter_mut().find(|t| t.root_id == payload.root_id)
                        {
                            thread.last_activity_at = payload.last_activity_at.clone();
                            if skip_fetch {
                                thread.message_count += 1;
                            }
                        }
                        next.sort_by(|a, b| b.last_activity_at.0.cmp(&a.last_activity_at.0));
                        next
                    });

                    if skip_fetch {
                        let mut updated = (*pending_activity).clone();
                        updated.remove(&payload.root_id);
                        pending_activity.set(updated);
                    } else if (*selected_thread).is_some_and(|id| id == payload.root_id) {
                        let messages = messages.clone();
                        let error = error.clone();
                        spawn_local(async move {
                            let client = RustyGPTClient::shared();
                            match client
                                .get_thread_tree(&payload.root_id, None, Some(200))
                                .await
                            {
                                Ok(tree) => {
                                    messages.set(tree.messages);
                                    error.set(None);
                                }
                                Err(err) => {
                                    error.set(Some(format!("Failed to refresh thread: {err}")));
                                }
                            }
                        });
                    }
                }
            }));
        event_source
            .add_event_listener_with_callback("thread.activity", listener.as_ref().unchecked_ref())
            .expect("thread.activity listener");
        listeners.borrow_mut().push(listener);
    }

    // message.delta
    {
        let selected_thread = selected_thread.clone();
        let typing = typing.clone();
        let typing_timer = typing_timer.clone();
        let streaming = streaming.clone();
        let listener =
            Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(move |event: MessageEvent| {
                if let Some(data) = event.data().as_string()
                    && let Ok(ConversationStreamEvent::MessageDelta { payload }) = from_str(&data)
                {
                    if let Some(chunk) = payload
                        .choices
                        .iter()
                        .filter_map(|choice| choice.delta.content.clone())
                        .reduce(|mut acc, part| {
                            acc.push_str(&part);
                            acc
                        })
                    {
                        streaming.set({
                            let mut next = (*streaming).clone();
                            next.entry(payload.message_id)
                                .and_modify(|entry| {
                                    entry.content.push_str(&chunk);
                                    entry.depth = payload.depth.unwrap_or(entry.depth);
                                    entry.parent_id = payload.parent_id;
                                    entry.root_id = payload.root_id;
                                    entry.conversation_id = payload.conversation_id;
                                })
                                .or_insert_with(|| StreamingEntry {
                                    message_id: payload.message_id,
                                    root_id: payload.root_id,
                                    parent_id: payload.parent_id,
                                    conversation_id: payload.conversation_id,
                                    depth: payload.depth.unwrap_or(1),
                                    content: chunk.clone(),
                                });
                            next
                        });
                    }

                    if (*selected_thread).is_some_and(|id| id == payload.root_id) {
                        typing.set(true);
                        {
                            let mut guard = typing_timer.borrow_mut();
                            if let Some(existing) = guard.take() {
                                existing.cancel();
                            }
                        }
                    }
                }
            }));
        event_source
            .add_event_listener_with_callback("message.delta", listener.as_ref().unchecked_ref())
            .expect("message.delta listener");
        listeners.borrow_mut().push(listener);
    }

    // message.done
    {
        let typing = typing.clone();
        let typing_timer = typing_timer.clone();
        let messages = messages.clone();
        let error = error.clone();
        let streaming = streaming.clone();
        let pending_activity = pending_activity.clone();
        let listener =
            Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(move |event: MessageEvent| {
                if let Some(data) = event.data().as_string()
                    && let Ok(ConversationStreamEvent::MessageDone { payload }) = from_str(&data)
                {
                    {
                        let mut guard = typing_timer.borrow_mut();
                        if let Some(existing) = guard.take() {
                            existing.cancel();
                        }
                    }
                    typing.set(false);
                    error.set(None);

                    let entry = {
                        let mut buffer = (*streaming).clone();
                        let removed = buffer.remove(&payload.message_id);
                        streaming.set(buffer);
                        removed
                    };

                    if let Some(entry) = entry {
                        messages.set({
                            let mut next = (*messages).clone();
                            if let Some(existing) =
                                next.iter_mut().find(|msg| msg.id == payload.message_id)
                            {
                                existing.content.clone_from(&entry.content);
                                existing.role = MessageRole::Assistant;
                            } else {
                                next.push(MessageView {
                                    id: payload.message_id,
                                    root_id: entry.root_id,
                                    parent_id: entry.parent_id,
                                    conversation_id: entry.conversation_id,
                                    author_user_id: None,
                                    role: MessageRole::Assistant,
                                    content: entry.content.clone(),
                                    path: String::new(),
                                    depth: entry.depth,
                                    created_at: Timestamp(Utc::now()),
                                });
                            }
                            next.sort_by(|a, b| a.created_at.0.cmp(&b.created_at.0));
                            next
                        });

                        pending_activity.set({
                            let mut roots = (*pending_activity).clone();
                            roots.insert(entry.root_id);
                            roots
                        });
                    }
                }
            }));
        event_source
            .add_event_listener_with_callback("message.done", listener.as_ref().unchecked_ref())
            .expect("message.done listener");
        listeners.borrow_mut().push(listener);
    }

    // typing.update
    {
        let selected_thread = selected_thread.clone();
        let typing = typing.clone();
        let typing_timer = typing_timer.clone();
        let listener =
            Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(move |event: MessageEvent| {
                if let Some(data) = event.data().as_string()
                    && let Ok(ConversationStreamEvent::TypingUpdate { payload }) = from_str(&data)
                    && payload.conversation_id == conversation_id
                    && (*selected_thread).is_some_and(|root| root == payload.root_id)
                {
                    let expires_at = payload.expires_at.0;
                    let now = Utc::now();
                    let remaining_ms = (expires_at - now).num_milliseconds();

                    if remaining_ms <= 0 {
                        {
                            let mut guard = typing_timer.borrow_mut();
                            if let Some(existing) = guard.take() {
                                existing.cancel();
                            }
                        }
                        typing.set(false);
                    } else {
                        let capped = remaining_ms.min(i64::from(u32::MAX));
                        let Ok(duration_ms) = u32::try_from(capped) else {
                            typing.set(false);
                            return;
                        };
                        let typing_clone = typing.clone();
                        let timer_handle = typing_timer.clone();
                        {
                            let mut guard = typing_timer.borrow_mut();
                            if let Some(existing) = guard.take() {
                                existing.cancel();
                            }
                            let timeout = Timeout::new(duration_ms, move || {
                                typing_clone.set(false);
                                timer_handle.borrow_mut().take();
                            });
                            *guard = Some(timeout);
                        }
                        typing.set(true);
                    }
                }
            }));
        event_source
            .add_event_listener_with_callback("typing.update", listener.as_ref().unchecked_ref())
            .expect("typing.update listener");
        listeners.borrow_mut().push(listener);
    }

    // presence.update
    {
        let online_users = online_users.clone();
        let listener =
            Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(move |event: MessageEvent| {
                if let Some(data) = event.data().as_string()
                    && let Ok(ConversationStreamEvent::PresenceUpdate { payload }) = from_str(&data)
                {
                    online_users.set({
                        let mut current = (*online_users).clone();
                        match payload.status {
                            PresenceStatus::Offline => {
                                current.remove(&payload.user_id);
                            }
                            _ => {
                                current.insert(payload.user_id);
                            }
                        }
                        current
                    });
                }
            }));
        event_source
            .add_event_listener_with_callback("presence.update", listener.as_ref().unchecked_ref())
            .expect("presence.update listener");
        listeners.borrow_mut().push(listener);
    }

    // unread.update
    {
        let unread_counts = unread_counts.clone();
        let listener =
            Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(move |event: MessageEvent| {
                if let Some(data) = event.data().as_string()
                    && let Ok(ConversationStreamEvent::UnreadUpdate { payload }) = from_str(&data)
                {
                    unread_counts.set({
                        let mut map = (*unread_counts).clone();
                        map.insert(payload.root_id, payload.unread);
                        map
                    });
                }
            }));
        event_source
            .add_event_listener_with_callback("unread.update", listener.as_ref().unchecked_ref())
            .expect("unread.update listener");
        listeners.borrow_mut().push(listener);
    }

    // membership.changed
    {
        let threads = threads.clone();
        let selected_thread = selected_thread.clone();
        let error = error.clone();
        let unread_counts = unread_counts.clone();
        let online_users = online_users.clone();
        let listener =
            Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(move |event: MessageEvent| {
                if let Some(data) = event.data().as_string()
                    && let Ok(ConversationStreamEvent::MembershipChanged { payload }) =
                        from_str(&data)
                    && payload.conversation_id == conversation_id
                {
                    if matches!(payload.action, MembershipChangeAction::Removed) {
                        online_users.set({
                            let mut set = (*online_users).clone();
                            set.remove(&payload.user_id);
                            set
                        });
                    }

                    let current_selected = *selected_thread;
                    let threads_state = threads.clone();
                    let selected_state = selected_thread.clone();
                    let error_state = error.clone();
                    let unread_state = unread_counts.clone();
                    spawn_local(async move {
                        let client = RustyGPTClient::shared();
                        match client.list_threads(&conversation_id, None, Some(50)).await {
                            Ok(mut response) => {
                                response.threads.sort_by(|a, b| {
                                    b.last_activity_at.0.cmp(&a.last_activity_at.0)
                                });
                                let threads_vec = response.threads.clone();
                                threads_state.set(threads_vec.clone());

                                if let Some(selected) = current_selected {
                                    if !threads_vec.iter().any(|item| item.root_id == selected) {
                                        let next = threads_vec.first().map(|item| item.root_id);
                                        selected_state.set(next);
                                    }
                                } else {
                                    let next = threads_vec.first().map(|item| item.root_id);
                                    selected_state.set(next);
                                }

                                match client.unread_summary(&conversation_id).await {
                                    Ok(summary) => {
                                        let map = summary
                                            .threads
                                            .into_iter()
                                            .map(|entry| (entry.root_id, entry.unread))
                                            .collect();
                                        unread_state.set(map);
                                        error_state.set(None);
                                    }
                                    Err(err) => {
                                        error_state.set(Some(format!(
                                            "Failed to refresh unread summary: {err}"
                                        )));
                                    }
                                }
                            }
                            Err(err) => {
                                error_state.set(Some(format!("Failed to refresh threads: {err}")));
                            }
                        }
                    });
                }
            }));
        event_source
            .add_event_listener_with_callback(
                "membership.changed",
                listener.as_ref().unchecked_ref(),
            )
            .expect("membership.changed listener");
        listeners.borrow_mut().push(listener);
    }

    // errors
    {
        let error = error.clone();
        let listener =
            Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(move |event: MessageEvent| {
                if let Some(data) = event.data().as_string()
                    && let Ok(ConversationStreamEvent::Error { payload }) = from_str(&data)
                {
                    error.set(Some(format!(
                        "Stream error {}: {}",
                        payload.code, payload.message
                    )));
                }
            }));
        event_source
            .add_event_listener_with_callback("error", listener.as_ref().unchecked_ref())
            .expect("error listener");
        listeners.borrow_mut().push(listener);
    }
}

#[derive(Clone, PartialEq, Eq)]
enum ComposerTarget {
    Root,
    Reply { parent_id: Uuid, root_id: Uuid },
}

#[function_component(ChatPage)]
pub fn chat_page(props: &ChatPageProps) -> Html {
    let conversation_uuid = props
        .conversation_id
        .as_ref()
        .and_then(|value| Uuid::parse_str(value).ok());

    let threads = use_state(Vec::<ThreadSummary>::new);
    let selected_thread = use_state(|| None::<Uuid>);
    let messages = use_state(Vec::<MessageView>::new);
    let composer_text = use_state(String::new);
    let composer_target = use_state(|| ComposerTarget::Root);
    let composer_busy = use_state(|| false);
    let typing_active = use_state(|| false);
    let typing_timer = use_mut_ref(|| None::<Timeout>);
    let streaming_buffers = use_state(HashMap::<Uuid, StreamingEntry>::new);
    let pending_activity_roots = use_state(HashSet::<Uuid>::new);
    let online_users = use_state(HashSet::<Uuid>::new);
    let unread_counts = use_state(HashMap::<Uuid, i64>::new);
    let error_message = use_state(|| None::<String>);

    // Refresh threads when the conversation changes
    {
        let conversation_id_prop = props.conversation_id.clone();
        let threads_handle = threads.clone();
        let selected_thread_handle = selected_thread.clone();
        let messages_handle = messages.clone();
        let composer_text_handle = composer_text.clone();
        let composer_target_handle = composer_target.clone();
        let streaming_handle = streaming_buffers.clone();
        let error_handle = error_message.clone();
        let unread_counts_handle = unread_counts.clone();
        let online_users_handle = online_users.clone();
        use_effect_with(conversation_id_prop, move |id_opt| {
            threads_handle.set(Vec::new());
            selected_thread_handle.set(None);
            messages_handle.set(Vec::new());
            composer_text_handle.set(String::new());
            composer_target_handle.set(ComposerTarget::Root);
            streaming_handle.set(HashMap::new());
            unread_counts_handle.set(HashMap::new());
            online_users_handle.set(HashSet::new());

            if let Some(id) = id_opt {
                match Uuid::parse_str(id) {
                    Ok(conv_id) => {
                        let threads = threads_handle.clone();
                        let selected_thread = selected_thread_handle.clone();
                        let composer_target = composer_target_handle.clone();
                        let error = error_handle.clone();
                        let unread_counts = unread_counts_handle;
                        spawn_local(async move {
                            let client = RustyGPTClient::shared();
                            match client.list_threads(&conv_id, None, Some(50)).await {
                                Ok(mut response) => {
                                    response.threads.sort_by(|a, b| {
                                        b.last_activity_at.0.cmp(&a.last_activity_at.0)
                                    });
                                    let first_root =
                                        response.threads.first().map(|item| item.root_id);
                                    threads.set(response.threads.clone());
                                    if let Some(root_id) = first_root {
                                        selected_thread.set(Some(root_id));
                                        composer_target.set(ComposerTarget::Reply {
                                            parent_id: root_id,
                                            root_id,
                                        });
                                    }

                                    match client.unread_summary(&conv_id).await {
                                        Ok(summary) => {
                                            let map = summary
                                                .threads
                                                .into_iter()
                                                .map(|entry| (entry.root_id, entry.unread))
                                                .collect();
                                            unread_counts.set(map);
                                            error.set(None);
                                        }
                                        Err(err) => {
                                            error.set(Some(format!(
                                                "Failed to load unread summary: {err}"
                                            )));
                                        }
                                    }
                                }
                                Err(err) => {
                                    error.set(Some(format!("Failed to load threads: {err}")));
                                }
                            }
                        });
                    }
                    Err(_) => {
                        error_handle.set(Some("Invalid conversation ID".to_string()));
                    }
                }
            }

            || ()
        });
    }

    // Load thread messages when selection changes
    {
        let messages_handle = messages.clone();
        let error_handle = error_message.clone();
        let composer_target_handle = composer_target.clone();
        let composer_text_handle = composer_text.clone();
        use_effect_with(*selected_thread, move |root_opt| {
            if let Some(root_id) = *root_opt {
                messages_handle.set(Vec::new());
                composer_text_handle.set(String::new());
                composer_target_handle.set(ComposerTarget::Reply {
                    parent_id: root_id,
                    root_id,
                });

                let messages = messages_handle.clone();
                let error = error_handle;
                let current_root = root_id;
                spawn_local(async move {
                    let client = RustyGPTClient::shared();
                    match client.get_thread_tree(&current_root, None, Some(200)).await {
                        Ok(tree) => {
                            messages.set(tree.messages);
                            error.set(None);
                        }
                        Err(err) => {
                            error.set(Some(format!("Failed to load thread: {err}")));
                        }
                    }
                });
            }

            || ()
        });
    }

    // Subscribe to SSE updates for the conversation
    {
        let conversation_id_prop = props.conversation_id.clone();
        let threads_handle = threads.clone();
        let selected_thread_handle = selected_thread.clone();
        let messages_handle = messages.clone();
        let error_handle = error_message.clone();
        let typing_handle = typing_active.clone();
        let typing_timer_handle = typing_timer;
        let streaming_handle = streaming_buffers.clone();
        let pending_activity_handle = pending_activity_roots;
        let online_users_handle = online_users.clone();
        let unread_counts_handle = unread_counts.clone();

        use_effect_with(conversation_id_prop, move |id_opt| {
            let mut cleanup: Option<(EventSource, ListenerRegistry)> = None;

            if let Some(id) = id_opt
                && let Ok(conv_id) = Uuid::parse_str(id)
            {
                let client = RustyGPTClient::shared();
                if let Ok(event_source) =
                    EventSource::new(&client.conversation_stream_url(&conv_id))
                {
                    let listeners: ListenerRegistry = Rc::new(RefCell::new(Vec::new()));

                    register_stream_listeners(
                        &event_source,
                        conv_id,
                        &listeners,
                        &threads_handle,
                        &selected_thread_handle,
                        &messages_handle,
                        &error_handle,
                        &typing_handle,
                        &typing_timer_handle,
                        &streaming_handle,
                        &pending_activity_handle,
                        &online_users_handle,
                        &unread_counts_handle,
                    );

                    cleanup = Some((event_source, listeners));
                }
            }

            move || {
                if let Some((event_source, listeners)) = cleanup {
                    event_source.close();
                    listeners.borrow_mut().clear();
                }
            }
        });
    }

    let on_select_thread = {
        let selected_thread = selected_thread.clone();
        Callback::from(move |root_id: Uuid| {
            selected_thread.set(Some(root_id));
        })
    };

    let on_new_thread = {
        let selected_thread = selected_thread.clone();
        let composer_target = composer_target.clone();
        let messages = messages.clone();
        let composer_text = composer_text.clone();
        Callback::from(move |_: yew::MouseEvent| {
            selected_thread.set(None);
            messages.set(Vec::new());
            composer_text.set(String::new());
            composer_target.set(ComposerTarget::Root);
        })
    };

    let on_reply_to_message = {
        let composer_target = composer_target.clone();
        let composer_text = composer_text.clone();
        Callback::from(move |message: MessageView| {
            composer_text.set(String::new());
            composer_target.set(ComposerTarget::Reply {
                parent_id: message.id,
                root_id: message.root_id,
            });
        })
    };

    let on_composer_text = {
        let composer_text = composer_text.clone();
        Callback::from(move |value: String| composer_text.set(value))
    };

    let on_cancel_reply = {
        let composer_target = composer_target.clone();
        let composer_text = composer_text.clone();
        let selected_thread = selected_thread.clone();
        Callback::from(move |()| {
            composer_text.set(String::new());
            if let Some(root_id) = *selected_thread {
                composer_target.set(ComposerTarget::Reply {
                    parent_id: root_id,
                    root_id,
                });
            } else {
                composer_target.set(ComposerTarget::Root);
            }
        })
    };

    let on_submit_message = {
        let conv_uuid = conversation_uuid;
        let composer_target = composer_target.clone();
        let composer_text = composer_text.clone();
        let composer_busy = composer_busy.clone();
        let selected_thread = selected_thread.clone();
        let messages = messages.clone();
        let error = error_message.clone();
        let typing = typing_active.clone();
        let threads_handle = threads.clone();
        Callback::from(move |()| {
            if *composer_busy {
                return;
            }

            let Some(conv_id) = conv_uuid else {
                error.set(Some("Conversation not selected".to_string()));
                return;
            };

            let trimmed = (*composer_text).trim();
            if trimmed.is_empty() {
                return;
            }

            composer_busy.set(true);

            match (*composer_target).clone() {
                ComposerTarget::Root => {
                    let composer_text = composer_text.clone();
                    let composer_busy = composer_busy.clone();
                    let selected_thread = selected_thread.clone();
                    let composer_target = composer_target.clone();
                    let messages = messages.clone();
                    let error = error.clone();
                    let text_to_send = trimmed.to_owned();
                    spawn_local(async move {
                        let client = RustyGPTClient::shared();
                        let request = PostRootMessageRequest {
                            content: text_to_send,
                            role: Some(MessageRole::User),
                        };
                        match client.post_root_message(&conv_id, &request).await {
                            Ok(response) => {
                                composer_text.set(String::new());
                                composer_busy.set(false);
                                composer_target.set(ComposerTarget::Reply {
                                    parent_id: response.root_id,
                                    root_id: response.root_id,
                                });
                                selected_thread.set(Some(response.root_id));
                                let messages = messages.clone();
                                let error = error.clone();
                                spawn_local(async move {
                                    let client = RustyGPTClient::shared();
                                    match client
                                        .get_thread_tree(&response.root_id, None, Some(200))
                                        .await
                                    {
                                        Ok(tree) => {
                                            messages.set(tree.messages);
                                            error.set(None);
                                        }
                                        Err(err) => {
                                            error.set(Some(format!(
                                                "Failed to load new thread: {err}"
                                            )));
                                        }
                                    }
                                });
                            }
                            Err(err) => {
                                composer_busy.set(false);
                                error.set(Some(format!("Failed to post message: {err}")));
                            }
                        }
                    });
                }
                ComposerTarget::Reply { parent_id, root_id } => {
                    let composer_text = composer_text.clone();
                    let composer_busy = composer_busy.clone();
                    let messages = messages.clone();
                    let error = error.clone();
                    let typing = typing.clone();
                    let threads_handle = threads_handle.clone();
                    let reply_content = trimmed.to_owned();
                    spawn_local(async move {
                        let client = RustyGPTClient::shared();
                        let request = ReplyMessageRequest {
                            content: reply_content,
                            role: Some(MessageRole::User),
                        };
                        match client.reply_message(&parent_id, &request).await {
                            Ok(_) => {
                                composer_text.set(String::new());
                                composer_busy.set(false);
                                typing.set(true);
                                let messages = messages.clone();
                                let error = error.clone();
                                let threads_handle = threads_handle.clone();
                                spawn_local(async move {
                                    let client = RustyGPTClient::shared();
                                    match client.get_thread_tree(&root_id, None, Some(200)).await {
                                        Ok(tree) => {
                                            messages.set(tree.messages.clone());
                                            error.set(None);
                                            typing.set(false);
                                            threads_handle.set({
                                                let mut next = (*threads_handle).clone();
                                                if let Some(summary) = next
                                                    .iter_mut()
                                                    .find(|item| item.root_id == root_id)
                                                {
                                                    summary.last_activity_at =
                                                        tree.messages.last().map_or_else(
                                                            || summary.last_activity_at.clone(),
                                                            |msg| msg.created_at.clone(),
                                                        );
                                                    summary.message_count += 1;
                                                }
                                                next.sort_by(|a, b| {
                                                    b.last_activity_at.0.cmp(&a.last_activity_at.0)
                                                });
                                                next
                                            });
                                        }
                                        Err(err) => {
                                            error.set(Some(format!(
                                                "Failed to refresh thread: {err}"
                                            )));
                                            typing.set(false);
                                        }
                                    }
                                });
                            }
                            Err(err) => {
                                composer_busy.set(false);
                                error.set(Some(format!("Failed to send reply: {err}")));
                            }
                        }
                    });
                }
            }
        })
    };

    let mut streaming_for_selected: Vec<StreamingDisplay> = {
        let selected = *selected_thread;
        (*streaming_buffers)
            .clone()
            .into_iter()
            .filter_map(|(message_id, entry)| {
                if Some(entry.root_id) == selected {
                    Some(StreamingDisplay {
                        message_id,
                        root_id: entry.root_id,
                        parent_id: entry.parent_id,
                        conversation_id: entry.conversation_id,
                        depth: entry.depth,
                        content: entry.content,
                    })
                } else {
                    None
                }
            })
            .collect()
    };
    streaming_for_selected.sort_by_key(|entry| entry.message_id);
    let typing_display = *typing_active || !streaming_for_selected.is_empty();

    let (placeholder, show_cancel) = match &*composer_target {
        ComposerTarget::Root => ("Start a new thread".to_string(), false),
        ComposerTarget::Reply { .. } => ("Reply in thread".to_string(), true),
    };

    let submit_label = match &*composer_target {
        ComposerTarget::Root => "Create Thread".to_string(),
        ComposerTarget::Reply { .. } => "Send Reply".to_string(),
    };

    let online_count = online_users.len();

    html! {
        <div class="h-full flex">
            <div class="w-full md:w-1/3 border-r border-base-300 flex flex-col">
                <div class="flex items-center justify-between p-3 border-b border-base-300">
                    <h2 class="font-semibold">{"Threads"}</h2>
                    <button class="btn btn-sm btn-primary" type="button" onclick={on_new_thread}> {"New Thread"} </button>
                </div>
                <div class="flex-1 overflow-y-auto">
                    <div class="px-3 py-2 text-xs text-base-content/60">
                        { format!("Participants online: {online_count}") }
                    </div>
                    <ThreadList
                        threads={(*threads).clone()}
                        selected={*selected_thread}
                        unread_counts={(*unread_counts).clone()}
                        on_select={on_select_thread}
                    />
                </div>
            </div>
            <div class="flex-1 flex flex-col">
                {
                    (*error_message)
                        .clone()
                        .map_or_else(
                            || html! {},
                            |error| html! { <div class="alert alert-error rounded-none">{ error }</div> },
                        )
                }
                <div class="flex-1 overflow-y-auto p-4 space-y-2">
                    <ThreadView
                        messages={(*messages).clone()}
                        streaming={streaming_for_selected.clone()}
                        on_reply={on_reply_to_message}
                    />
                    <TypingIndicator active={typing_display} />
                </div>
                <div class="border-t border-base-300 p-4 bg-base-200">
                    <ThreadComposer
                        text={(*composer_text).clone()}
                        on_text_change={on_composer_text}
                        on_submit={on_submit_message}
                        disabled={*composer_busy}
                        placeholder={placeholder}
                        submit_label={submit_label}
                        show_cancel={show_cancel}
                        on_cancel={on_cancel_reply}
                    />
                </div>
            </div>
        </div>
    }
}
