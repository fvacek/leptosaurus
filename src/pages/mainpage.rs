use std::pin::Pin;
use futures::{Stream, StreamExt};
use crate::web_sys::CloseEvent;
use leptos::ev::Event;
use leptos::*;
use leptos_router::{use_query_map};
use leptos_use::core::ConnectionReadyState;
use leptos_use::{use_websocket_with_options, UseWebSocketOptions};
use shv::client::LoginParams;
use shv::util::{login_from_url, sha1_password_hash};
use shv::{RpcMessage, RpcMessageMetaTags, RpcValue};
use url::Url;

#[component]
pub fn MainPage() -> impl IntoView {

    let (is_connect, set_connect) = create_signal(false);
    let (is_socket_connected, set_socket_connected) = create_signal(false);

    let input_url: NodeRef<html::Input> = create_node_ref();
    let url_str = create_rw_signal(String::from("wss://nirvana.elektroline.cz:37778"));
    let query = use_query_map();
    create_effect(move |_| {
        query.with(|q| {
            let qs = q.to_query_string();
            if qs.is_empty() {
                url_str.update(|url: &mut String| { url.push_str("/?user=test&password=test"); })
            } else {
                url_str.update(|url: &mut String| {
                    url.push('/');
                    url.push_str(&qs);
                })
            };
        });
    });
    view! {
        <div>
            <h2>"Leptosaurus"</h2>
            <p>Leptos + websocket + SHV playground</p>
            // <p>{url_str}</p>
            <div style="display: flex; flex-direction: row; align-items: center; gap: 1em;">
                <label for="url-str">"Url"</label>
                <div style="flex-grow:100">
                    <input type="text" style="width:100%" prop:value=url_str node_ref=input_url/>
                </div>
            </div>
            <hr/>
            <div style="display: flex; flex-direction: row; align-items: center; gap: 1em;">
                <span>"socket connected: " {move || is_socket_connected}</span>
                <button on:click=move|_| set_connect.set(true) disabled=Signal::derive(move|| is_connect.get())>"Connect"</button>
                <button on:click=move|_| set_connect.set(false) disabled=Signal::derive(move|| !is_connect.get())>"Disconnect"</button>
            </div>
            <hr/>
            {move || if is_connect() {
                let url_str = url_str.get_untracked();
                view! {
                    <Wss url=url_str socket_connected=set_socket_connected />
                }
            } else {
                "Websocket is closed".into_view()
            }}
        </div>
    }
}
#[component]
fn Wss(
    #[prop(into)] url: String,
    #[prop(into)] socket_connected: WriteSignal<bool>,
) -> impl IntoView {
    let (log, set_log) = create_signal(vec![]);
    let (ws_connected, set_ws_connected) = create_signal(false);

    fn append_log(log: &WriteSignal<Vec<String>>, message: String) {
        let _ = log.update(|log: &mut Vec<_>| log.push(message));
    }

    let on_open_callback = move |e: Event| {
        set_log.update(|log: &mut Vec<_>| log.push(format! {"[onopen]: event {:?}", e.type_()}));
        socket_connected.set(true);
        set_ws_connected(true);
    };

    let on_close_callback = move |e: CloseEvent| {
        set_log.update(|log: &mut Vec<_>| log.push(format! {"[onclose]: event {:?}", e.type_()}));
        socket_connected.set(false);
        set_ws_connected(false);
    };

    let on_error_callback = move |e: Event| {
        set_log.update(|log: &mut Vec<_>| log.push(format! {"[onerror]: event {:?}", e.type_()}));
        socket_connected.set(false);
        set_ws_connected(false);
    };

    //let on_message_callback = move |m: String| {
    //    set_log.update(|log: &mut Vec<_>| {
    //        log.push(format! {"[onmessage]: {}", m})
    //    });
    //};

    //let on_message_bytes_callback = move |m: Vec<u8>| {
    //    set_log.update(|log: &mut Vec<_>| {
    //        log.push(format! {"[onmessage-data]: {:?}", m})
    //    });
    //};

    let ws = use_websocket_with_options(
        &url,
        UseWebSocketOptions::default()
            .immediate(true)
            .on_open(on_open_callback.clone())
            .on_close(on_close_callback.clone())
            .on_error(on_error_callback.clone()), //.on_message(on_message_callback.clone())
                                                  //.on_message_bytes(on_message_bytes_callback.clone()),
    );
    //let (broker_connected, set_broker_connected) = create_signal(false);
    let (message_to_send, set_message_to_send) = create_signal(RpcMessage::default());
    let (received_message, set_received_message) = create_signal(RpcMessage::default());
    create_effect(move |_| {
        if let Some(data) = ws.message_bytes.get() {
            let frame = shv::streamrw::read_frame(&data).expect("valid response frame");
            let rpc_msg = frame.to_rpcmesage().expect("valid response message");
            // append_log(&set_log, format! {"[message_bytes]: {:?}", &data});
            append_log(&set_log, format! {"[received]: {}", rpc_msg.to_cpon()});
            set_received_message(rpc_msg);
        };
    });
    create_effect(move |_| {
        let rq = message_to_send();
        if rq.request_id().is_none() {
            return;
        }
        append_log(&set_log, format! {"[sent]: {}", rq.to_cpon()});
        let frame = rq.to_frame().expect("valid hello message");
        let mut buff: Vec<u8> = vec![];
        shv::streamrw::write_frame(&mut buff, frame).expect("valid frame");
        (ws.send_bytes)(buff);
    });
    let url = Url::parse(&url).expect("valid url");
    let (user, password) = login_from_url(&url);
    let login_resource = create_resource(
        ws_connected,
 move|is_connected| {
            let user = user.clone();
            let password = password.clone();
            async move {
                if !is_connected {
                    return Err("Not web socket is not connected.".to_string())
                }
                let mut receive_stream = received_message.to_stream();
                async fn call_method(shv_path: &str, method: &str, param: Option<RpcValue>,
                                     send: WriteSignal<RpcMessage>,
                                     receive_stream: &mut Pin<Box<dyn Stream<Item=RpcMessage>>>) -> Result<RpcValue, String>
                {
                    let rq = RpcMessage::new_request(shv_path, method, param);
                    let rq_id = rq.request_id().unwrap_or_default();
                    send(rq);
                    //set_login_status.set(crate::pages::mainpage::LoginStatus::Login);
                    while let Some(msg) = receive_stream.next().await {
                        if msg.request_id().unwrap_or_default() == rq_id {
                            match msg.result() {
                                Ok(val) => { return Ok(val.clone()) }
                                Err(err) => { return Err(err.to_string()) }
                            }
                        }
                        //return Err("Unexpected end of stream".to_string());
                    };
                    Err("NP".to_string())
                }

                let result = match call_method("","hello", None, set_message_to_send, &mut receive_stream).await {
                    Ok(v) => { v }
                    Err(e) => { return Err(e) }
                };
                let nonce = result
                    .as_map()
                    .get("nonce")
                    .expect("nonce")
                    .as_str();
                let hash = sha1_password_hash(password.as_bytes(), nonce.as_bytes());
                let hashed_password = std::str::from_utf8(&hash).expect("ascii7 string").into();
                let login_params = LoginParams {
                    user,
                    password: hashed_password,
                    reset_session: false,
                    ..Default::default()
                };
                match call_method("","login", Some(login_params.to_rpcvalue()), set_message_to_send, &mut receive_stream).await {
                    Ok(_) => { Ok(()) }
                    Err(e) => { Err(e) }
                }
            }
        }
    );
    #[derive(Clone, PartialEq)]
    struct RpcCall {
        shv_path: String,
        method: String,
        param: String,
    }
    let (method_call, set_method_call) = create_signal(RpcCall {
        shv_path: "".to_string(),
        method: "".to_string(),
        param: "".to_string(),
    });
    let call_method_resource = create_resource(
        method_call,
        move |params| async move {
            let param = if params.param.is_empty() {
                None
            } else {
                Some(RpcValue::from_cpon(&params.param).expect("Valid cpon"))
            };
            let rq = RpcMessage::new_request(&params.shv_path, &params.method, param);
            let rq_id = rq.request_id().expect("request id");
            set_message_to_send(rq);
            let mut stream = received_message.to_stream();
            while let Some(msg) = stream.next().await {
                if msg.request_id().unwrap_or_default() == rq_id {
                    return msg;
                }
            }
            return RpcMessage::default()
        }
    );

    let status = move || ws.ready_state.get().to_string();

    create_effect(move |_| {
        if let Some(m) = ws.message.get() {
            append_log(&set_log, format! {"[message]: {:?}", m});
        };
    });

    let connected = move || ws.ready_state.get() == ConnectionReadyState::Open;

    let input_shvpath: NodeRef<html::Input> = create_node_ref();
    let input_method: NodeRef<html::Input> = create_node_ref();
    let input_param: NodeRef<html::Input> = create_node_ref();

    view! {
        <div class="container">
            <div>
                <h2>"Websocket"</h2>
                <p>"status: " {status}</p>
                {move || match login_resource.get() {
                    None => view! { <p>"Broker login in process ..."</p> }.into_view(),
                    Some(Ok(())) => view! {
                        <div style="display: flex; flex-direction: row; align-items: center; gap: 1em;">
                            "shv path"
                            <input type="text" size="50" value="" node_ref=input_shvpath/>
                            "method"
                            <input type="text" value="dir" node_ref=input_method/>
                            "param"
                            <input type="text" value="" node_ref=input_param/>

                            <button
                                on:click= move |_| {
                                    set_method_call(RpcCall{
                                        shv_path: input_shvpath.get().unwrap().value(),
                                        method: input_method.get().unwrap().value(),
                                        param: input_param.get().unwrap().value(),
                                    });
                                }
                                disabled=move || !connected()
                            >
                                "Send"
                            </button>
                        </div>
                        <h3>"Result"</h3>
                        <textarea id="result" name="result" rows="5" cols="80">
                            {move || {
                                call_method_resource().map(|val| {
                                    match val.result() {
                                        Ok(val) => { val.to_cpon_indented("  ") }
                                        Err(err) => { err.to_string() }
                                    }
                                })
                            }}
                        </textarea>
                    }.into_view(),
                    Some(Err(err)) => view! { <p>"Login error " {err}</p> }.into_view(),
                }}
                <div>
                    <h3>"Log"</h3>
                    <button
                        on:click=move |_| set_log.set(vec![])
                        disabled=move || log.get().len() <= 0
                    >
                        "Clear"
                    </button>
                </div>
                <For
                    each=move || log.get().into_iter().enumerate()
                    key=|(index, _)| *index
                    let:item
                >
                    <div>{item.1}</div>
                </For>
            </div>
        </div>
    }
}
