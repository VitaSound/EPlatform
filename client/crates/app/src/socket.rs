//! Minimal Phoenix Channels v2 client.
//!
//! Browser implementation speaks the JSON wire format directly over
//! `web_sys::WebSocket`: `[join_ref, msg_ref, topic, event, payload]`.
//! A native no-op stub keeps `cargo check`/tests working outside wasm.

#[derive(Debug, Clone)]
pub enum SocketEvent {
    Opened,
    Joined,
    /// `phx_reply` payload for a non-join request (e.g. `start_app`).
    Reply(serde_json::Value),
    Command(serde_json::Value),
    Closed,
    Error(String),
}

/// Pure Phoenix Channels v2 wire-format helpers (no wasm dependencies),
/// extracted so the protocol can be unit-tested on the native target.
pub(crate) mod protocol {
    use serde_json::Value;

    /// `[join_ref, msg_ref, topic, "phx_join", payload]`
    pub fn join_frame(topic: &str) -> String {
        serde_json::json!(["1", "1", topic, "phx_join", {}]).to_string()
    }

    /// `[join_ref, msg_ref, topic, event, payload]`
    pub fn event_frame(ref_id: &str, topic: &str, event: &str, payload: &Value) -> String {
        serde_json::json!(["1", ref_id, topic, event, payload]).to_string()
    }

    /// Heartbeat goes to the special `phoenix` topic with a null join_ref.
    pub fn heartbeat_frame(ref_id: &str) -> String {
        serde_json::json!([null, ref_id, "phoenix", "phxheartbeat", {}]).to_string()
    }

    #[derive(Debug, PartialEq)]
    pub enum Decoded {
        /// First `phx_reply` with `status: ok` for our topic → join ack.
        Joined,
        /// A non-join `phx_reply` payload (request reply).
        Reply(Value),
        /// Server-initiated `command` push (createForm, setProperty, ...).
        Command(Value),
        /// Anything else (heartbeat replies, non-ok replies, foreign topics).
        Other,
    }

    /// Decode one incoming wire frame. Returns `None` for malformed input.
    pub fn decode(text: &str, topic: &str) -> Option<Decoded> {
        let val: Value = serde_json::from_str(text).ok()?;
        let arr = val.as_array()?;
        if arr.len() < 5 {
            return None;
        }
        let event = arr[3].as_str()?;
        let payload = arr[4].clone();
        Some(match event {
            "phx_reply"
                if arr[2].as_str() == Some(topic)
                    && payload.get("status").and_then(Value::as_str) == Some("ok") =>
            {
                // The join reply is the only reply with ref "1"; later
                // request replies (start_app/start_dialog) must be delivered
                // to the app so it can remember the main session id.
                if arr[1].as_str() == Some("1") {
                    Decoded::Joined
                } else {
                    Decoded::Reply(payload)
                }
            }
            "command" => Decoded::Command(payload),
            _ => Decoded::Other,
        })
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::SocketEvent;
    use serde_json::Value;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::sync::mpsc::Sender;
    use wasm_bindgen::{closure::Closure, JsCast};
    use wasm_bindgen_futures::spawn_local;

    #[derive(Clone)]
    pub struct PhoenixSocket {
        ws: Rc<web_sys::WebSocket>,
        topic: Rc<String>,
        next_ref: Rc<RefCell<u32>>,
    }

    impl PhoenixSocket {
        /// `wake` must call `Window::request_redraw` (or equivalent). Without
        /// it, socket messages sit in the mpsc queue until some other event
        /// wakes winit — idle dirty-paint no longer spins every frame.
        pub fn open(
            url: &str,
            topic: String,
            on_event: Sender<SocketEvent>,
            wake: Option<Rc<dyn Fn()>>,
        ) -> Self {
            let ws = web_sys::WebSocket::new(url).expect("failed to open WebSocket");
            let topic_rc = Rc::new(topic);
            let next_ref = Rc::new(RefCell::new(1u32));
            fn poke(wake: &Option<Rc<dyn Fn()>>) {
                if let Some(wake) = wake {
                    wake();
                }
            }

            // onopen → phx_join
            {
                let ws_open = ws.clone();
                let topic = topic_rc.clone();
                let on_event = on_event.clone();
                let wake = wake.clone();
                let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |_e| {
                    let _ =
                        ws_open.send_with_str(&crate::socket::protocol::join_frame(topic.as_str()));
                    let _ = on_event.send(SocketEvent::Opened);
                    poke(&wake);
                });
                ws.set_onopen(Some(closure.as_ref().unchecked_ref()));
                closure.forget();
            }

            // onmessage → join replies + server commands
            {
                let topic = topic_rc.clone();
                let joined = Cell::new(false);
                let on_event = on_event.clone();
                let wake = wake.clone();
                let closure = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(
                    move |e: web_sys::MessageEvent| {
                        let Some(text) = e.data().as_string() else {
                            return;
                        };
                        let mut woke = false;
                        match crate::socket::protocol::decode(&text, topic.as_str()) {
                            Some(crate::socket::protocol::Decoded::Joined) if !joined.get() => {
                                joined.set(true);
                                let _ = on_event.send(SocketEvent::Joined);
                                woke = true;
                            }
                            Some(crate::socket::protocol::Decoded::Reply(payload)) => {
                                let _ = on_event.send(SocketEvent::Reply(payload));
                                woke = true;
                            }
                            Some(crate::socket::protocol::Decoded::Command(payload)) => {
                                let _ = on_event.send(SocketEvent::Command(payload));
                                woke = true;
                            }
                            _ => {}
                        }
                        if woke {
                            poke(&wake);
                        }
                    },
                );
                ws.set_onmessage(Some(closure.as_ref().unchecked_ref()));
                closure.forget();
            }

            // onclose / onerror
            {
                let on_event = on_event.clone();
                let wake = wake.clone();
                let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |_e| {
                    let _ = on_event.send(SocketEvent::Closed);
                    poke(&wake);
                });
                ws.set_onclose(Some(closure.as_ref().unchecked_ref()));
                closure.forget();
            }
            {
                let on_event = on_event.clone();
                let wake = wake.clone();
                let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |_e| {
                    let _ = on_event.send(SocketEvent::Error("websocket error".into()));
                    poke(&wake);
                });
                ws.set_onerror(Some(closure.as_ref().unchecked_ref()));
                closure.forget();
            }

            // heartbeat so the server can detect dead connections
            {
                let ws = ws.clone();
                let next_ref = next_ref.clone();
                spawn_local(async move {
                    loop {
                        gloo_timers::future::TimeoutFuture::new(25_000).await;
                        let ref_str = {
                            let mut n = next_ref.borrow_mut();
                            *n += 1;
                            n.to_string()
                        };
                        let frame = crate::socket::protocol::heartbeat_frame(&ref_str);
                        if ws.send_with_str(&frame).is_err() {
                            break;
                        }
                    }
                });
            }

            Self {
                ws: Rc::new(ws),
                topic: topic_rc,
                next_ref,
            }
        }

        pub fn send(&self, event: &str, payload: Value) {
            let ref_str = {
                let mut n = self.next_ref.borrow_mut();
                *n += 1;
                n.to_string()
            };
            let frame = crate::socket::protocol::event_frame(
                &ref_str,
                self.topic.as_str(),
                event,
                &payload,
            );
            if let Err(e) = self.ws.send_with_str(&frame) {
                web_sys::console::log_1(&format!("ws send failed: {e:?}").into());
            }
        }

        pub fn close(&self) {
            let _ = self.ws.close();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::SocketEvent;
    use serde_json::Value;
    use std::sync::mpsc::Sender;

    /// Native stub: no connection, logs sends. Useful for `cargo test`.
    #[derive(Clone)]
    pub struct PhoenixSocket;

    impl PhoenixSocket {
        pub fn open(
            url: &str,
            topic: String,
            on_event: Sender<SocketEvent>,
            _wake: Option<std::rc::Rc<dyn Fn()>>,
        ) -> Self {
            let _ = (url, topic, on_event);
            Self
        }

        pub fn send(&self, event: &str, payload: Value) {
            println!("[native stub] {event}: {payload}");
        }

        pub fn close(&self) {}
    }
}

pub use imp::PhoenixSocket;

#[cfg(test)]
mod tests {
    use crate::socket::protocol::{self, Decoded};
    use serde_json::{json, Value};

    #[test]
    fn join_frame_is_v2_wire_format() {
        let frame: Value = serde_json::from_str(&protocol::join_frame("app:demo")).unwrap();
        assert_eq!(frame, json!(["1", "1", "app:demo", "phx_join", {}]));
    }

    #[test]
    fn event_frame_carries_join_ref_topic_and_payload() {
        let frame: Value = serde_json::from_str(&protocol::event_frame(
            "3",
            "app:demo",
            "send_event",
            &json!({"expr": "1+1"}),
        ))
        .unwrap();
        assert_eq!(
            frame,
            json!(["1", "3", "app:demo", "send_event", {"expr": "1+1"}])
        );
    }

    #[test]
    fn heartbeat_targets_phoenix_topic_with_null_join_ref() {
        let frame: Value = serde_json::from_str(&protocol::heartbeat_frame("7")).unwrap();
        assert_eq!(frame, json!([null, "7", "phoenix", "phxheartbeat", {}]));
    }

    #[test]
    fn decode_join_ack() {
        let frame = json!(["1", "1", "app:demo", "phx_reply", {"status": "ok"}]).to_string();
        assert_eq!(protocol::decode(&frame, "app:demo"), Some(Decoded::Joined));
    }

    #[test]
    fn decode_request_reply_after_join() {
        let frame = json!(["1", "2", "app:demo", "phx_reply",
            {"status": "ok", "response": {"form_id": "main", "role": "main"}}])
        .to_string();
        match protocol::decode(&frame, "app:demo") {
            Some(Decoded::Reply(payload)) => {
                assert_eq!(payload["response"]["role"], "main");
            }
            other => panic!("expected Reply, got {:?}", other),
        }
    }

    #[test]
    fn decode_command_push() {
        let frame = json!([
            "1",
            "2",
            "app:demo",
            "command",
            {"command": "createForm", "form_id": "f1"}
        ])
        .to_string();
        match protocol::decode(&frame, "app:demo") {
            Some(Decoded::Command(payload)) => {
                assert_eq!(payload["command"], "createForm");
                assert_eq!(payload["form_id"], "f1");
            }
            other => panic!("expected Command, got {other:?}"),
        }
    }

    #[test]
    fn decode_ignores_foreign_topic_and_non_ok_replies() {
        let heartbeat_reply =
            json!([null, "7", "phoenix", "phx_reply", {"status": "ok"}]).to_string();
        assert_eq!(
            protocol::decode(&heartbeat_reply, "app:demo"),
            Some(Decoded::Other)
        );

        let error_reply =
            json!(["1", "1", "app:demo", "phx_reply", {"status": "error"}]).to_string();
        assert_eq!(
            protocol::decode(&error_reply, "app:demo"),
            Some(Decoded::Other)
        );
    }

    #[test]
    fn decode_rejects_malformed_frames() {
        assert_eq!(protocol::decode("not json", "t"), None);
        assert_eq!(protocol::decode("[1,2]", "t"), None);
        assert_eq!(protocol::decode(r#"["1","1","t"]"#, "t"), None);
        assert_eq!(protocol::decode(r#"{"event":"x"}"#, "t"), None);
    }
}
