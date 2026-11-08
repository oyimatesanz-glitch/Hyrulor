use std::time::Duration;

use cosmic::iced::futures::{self, SinkExt};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{time, window::Id, Limits, Subscription};
use cosmic::prelude::*;
use cosmic::widget;
use serde_json::{json, Value};

use crate::socket::{self, AuthRequest, SocketEvent};
use crate::ui::{self, ChatEntry, ChatStatus};

const APP_ID: &str = "com.github.oyimatesanz-glitch.hyrulor-applet";
const POPUP_WIDTH: f32 = 420.0;
const POPUP_HEIGHT: f32 = 620.0;

#[derive(Debug, Clone)]
pub struct AuthField {
    pub name: String,
    pub label: String,
    pub required: bool,
    pub value: String,
}

/// Authorization form shown inside the popup. Secure mode renders every field;
/// simple mode only asks for a yes/no confirmation.
#[derive(Debug, Clone)]
pub struct AuthPrompt {
    pub mode: String,
    pub request_id: String,
    pub question: String,
    pub proposed_action: String,
    pub authorization_code: Option<String>,
    pub fields: Vec<AuthField>,
    pub error: Option<String>,
}

impl AuthPrompt {
    fn from_request(request: AuthRequest) -> Self {
        let form = request.form;
        let question = form
            .get("question")
            .and_then(Value::as_str)
            .unwrap_or("¿Seguro que quieres hacer esto?")
            .to_owned();
        let proposed_action = form
            .get("proposed_action")
            .map(|value| serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()))
            .unwrap_or_default();
        let authorization_code = form
            .get("authorization_code")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let fields = form
            .get("fields")
            .and_then(Value::as_array)
            .map(|fields| {
                fields
                    .iter()
                    .filter_map(|field| {
                        let name = field.get("name")?.as_str()?.to_owned();
                        let label = field
                            .get("label")
                            .and_then(Value::as_str)
                            .unwrap_or(name.as_str())
                            .to_owned();
                        let required = field
                            .get("required")
                            .and_then(Value::as_bool)
                            .unwrap_or(true);
                        Some(AuthField {
                            name,
                            label,
                            required,
                            value: String::new(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Self {
            mode: request.mode,
            request_id: request.request_id,
            question,
            proposed_action,
            authorization_code,
            fields,
            error: None,
        }
    }

    fn form_value(&self) -> Value {
        let mut values = serde_json::Map::new();
        for field in &self.fields {
            values.insert(
                field.name.clone(),
                Value::String(field.value.trim().to_owned()),
            );
        }
        Value::Object(values)
    }

    fn validate(&self) -> Result<(), String> {
        if self.mode != "secure" {
            return Ok(());
        }

        for field in &self.fields {
            if field.required && field.value.trim().is_empty() {
                return Err(format!("El campo «{}» es obligatorio.", field.label));
            }
        }

        if let Some(expected) = &self.authorization_code {
            let provided = self
                .fields
                .iter()
                .find(|field| field.name == "authorization_code")
                .map(|field| field.value.trim())
                .unwrap_or("");
            if provided != expected {
                return Err("Código de autorización incorrecto.".to_owned());
            }
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct AppModel {
    core: cosmic::Core,
    popup: Option<Id>,
    input: String,
    messages: Vec<ChatEntry>,
    status: ChatStatus,
    tool_label: Option<String>,
    socket_error: Option<String>,
    last_query: Option<String>,
    pending_answer: Option<usize>,
    pending_auth: Option<AuthPrompt>,
    secure_mode: Option<bool>,
    pulse: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    InputChanged(String),
    QuickQuery(String),
    Send,
    Retry,
    Pulse,
    AuthFieldChanged(usize, String),
    AuthSubmit,
    AuthCancel,
    ModeLoaded(bool),
    ToggleMode,
    Socket(SocketEvent),
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let mode_stream = cosmic::iced::stream::channel(
            4,
            |mut sender: futures::channel::mpsc::Sender<bool>| async move {
                if let Ok(secure) = socket::secure_mode().await {
                    let _ = sender.send(secure).await;
                }
            },
        );
        let mode_task = Task::run(mode_stream, |secure| {
            cosmic::Action::App(Message::ModeLoaded(secure))
        });

        (
            Self {
                core,
                ..Self::default()
            },
            mode_task,
        )
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let monogram = widget::text("H").size(19);

        cosmic::iced::widget::button(monogram)
            .on_press(Message::TogglePopup)
            .padding([3, 7])
            .into()
    }

    fn view_window(&self, id: Id) -> Element<'_, Self::Message> {
        if self.popup == Some(id) {
            let content = ui::view(
                &self.input,
                &self.messages,
                self.status,
                self.tool_label.as_deref(),
                self.socket_error.as_deref(),
                self.last_query.is_some(),
                self.pending_auth.as_ref(),
                self.secure_mode,
            );
            self.core.applet.popup_container(content).into()
        } else {
            widget::text("").into()
        }
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::TogglePopup => {
                return if let Some(popup) = self.popup.take() {
                    destroy_popup(popup)
                } else {
                    let popup_id = Id::unique();
                    self.popup = Some(popup_id);

                    let mut settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        popup_id,
                        Some((POPUP_WIDTH as u32, POPUP_HEIGHT as u32)),
                        None,
                        None,
                    );
                    settings.positioner.size_limits = Limits::NONE
                        .min_width(POPUP_WIDTH)
                        .min_height(POPUP_HEIGHT)
                        .max_width(POPUP_WIDTH)
                        .max_height(POPUP_HEIGHT);
                    get_popup(settings)
                };
            }
            Message::PopupClosed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                }
            }
            Message::InputChanged(value) => {
                self.input = value;
            }
            Message::QuickQuery(query) => {
                if self.status != ChatStatus::Thinking
                    && self.status != ChatStatus::Tool
                    && self.pending_auth.is_none()
                {
                    self.input.clear();
                    return self.begin_request(query, true);
                }
            }
            Message::Send => {
                let query = self.input.trim().to_owned();
                if !query.is_empty()
                    && self.status != ChatStatus::Thinking
                    && self.status != ChatStatus::Tool
                    && self.pending_auth.is_none()
                {
                    self.input.clear();
                    return self.begin_request(query, true);
                }
            }
            Message::Retry => {
                if self.pending_auth.is_none() {
                    if let Some(query) = self.last_query.clone() {
                        return self.begin_request(query, false);
                    }
                }
            }
            Message::Pulse => {
                self.pulse = !self.pulse;
            }
            Message::AuthFieldChanged(index, value) => {
                if let Some(prompt) = self.pending_auth.as_mut() {
                    if let Some(field) = prompt.fields.get_mut(index) {
                        field.value = value;
                    }
                    prompt.error = None;
                }
            }
            Message::AuthSubmit => {
                if let Some(prompt) = self.pending_auth.clone() {
                    if let Err(error) = prompt.validate() {
                        if let Some(current) = self.pending_auth.as_mut() {
                            current.error = Some(error);
                        }
                        return Task::none();
                    }

                    let form = if prompt.mode == "secure" {
                        Some(prompt.form_value())
                    } else {
                        None
                    };
                    return self.begin_authorize(prompt.request_id, true, form);
                }
            }
            Message::AuthCancel => {
                if let Some(prompt) = self.pending_auth.take() {
                    return self.begin_authorize(prompt.request_id, false, None);
                }
            }
            Message::ModeLoaded(secure) => {
                self.secure_mode = Some(secure);
            }
            Message::ToggleMode => {
                if self.pending_auth.is_some()
                    || self.status == ChatStatus::Thinking
                    || self.status == ChatStatus::Tool
                {
                    return Task::none();
                }

                let target = !self.secure_mode.unwrap_or(false);
                let stream = cosmic::iced::stream::channel(
                    4,
                    move |mut sender: futures::channel::mpsc::Sender<bool>| async move {
                        if let Ok(secure) = socket::set_secure_mode(target).await {
                            let _ = sender.send(secure).await;
                        }
                    },
                );
                return Task::run(stream, |secure| {
                    cosmic::Action::App(Message::ModeLoaded(secure))
                });
            }
            Message::Socket(event) => match event {
                SocketEvent::ToolCall(label) => {
                    self.status = ChatStatus::Tool;
                    self.tool_label = Some(label.clone());
                    if let Some(index) = self.pending_answer {
                        self.messages.insert(index, ChatEntry::tool(label));
                        self.pending_answer = Some(index + 1);
                    }
                }
                SocketEvent::Chunk(chunk) => {
                    if let Some(index) = self.pending_answer {
                        if let Some(entry) = self.messages.get_mut(index) {
                            entry.body.push_str(&chunk);
                        }
                    }
                }
                SocketEvent::Finished(answer) => {
                    if let Some(index) = self.pending_answer {
                        if let Some(entry) = self.messages.get_mut(index) {
                            // The terminal frame may contain the complete answer even
                            // when intermediate chunks have already been rendered.
                            if !answer.is_empty() {
                                entry.body = answer;
                            }
                        }
                    }
                    self.pending_answer = None;
                    self.status = ChatStatus::Idle;
                    self.tool_label = None;
                    self.socket_error = None;
                }
                SocketEvent::RequiresAuth(request) => {
                    if let Some(index) = self.pending_answer.take() {
                        if let Some(entry) = self.messages.get_mut(index) {
                            entry.body = "⏳ Se requiere autorización humana.".to_owned();
                        }
                    }
                    self.pending_auth = Some(AuthPrompt::from_request(request));
                    self.status = ChatStatus::Idle;
                    self.tool_label = None;
                    self.socket_error = None;
                }
                SocketEvent::Transcribed(text) => {
                    if let Some(index) = self.pending_answer.take() {
                        if index + 1 == self.messages.len() {
                            self.messages.remove(index);
                        }
                    }
                    self.status = ChatStatus::Idle;
                    self.tool_label = None;

                    if text.trim().is_empty() {
                        self.socket_error = Some("No se reconoció voz en el audio.".to_owned());
                    } else {
                        // Continue the conversation with the transcribed text.
                        return self.begin_request(text, true);
                    }
                }
                SocketEvent::Spoken(audio) => {
                    if let Some(index) = self.pending_answer.take() {
                        if let Some(entry) = self.messages.get_mut(index) {
                            entry.body = match audio {
                                Some(path) => format!("🔊 Audio generado: {path}"),
                                None => "🔊 Respuesta reproducida.".to_owned(),
                            };
                        }
                    }
                    self.status = ChatStatus::Idle;
                    self.tool_label = None;
                }
                SocketEvent::Failed(error) => {
                    if let Some(index) = self.pending_answer {
                        if let Some(entry) = self.messages.get_mut(index) {
                            if entry.body.is_empty() {
                                entry.body = "No pude completar esa tarea.".to_owned();
                            }
                        }
                    }
                    self.pending_answer = None;
                    self.status = ChatStatus::Error;
                    self.tool_label = None;
                    self.socket_error = Some(error);
                }
            },
        }

        Task::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        if self.status == ChatStatus::Thinking {
            time::every(Duration::from_millis(900)).map(|_| Message::Pulse)
        } else {
            Subscription::none()
        }
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

impl AppModel {
    fn begin_request(
        &mut self,
        query: String,
        add_user_message: bool,
    ) -> Task<cosmic::Action<Message>> {
        if let Some((label, request)) = sense_request(&query) {
            return self.begin_sense(query, label, request, add_user_message);
        }

        if add_user_message {
            self.messages.push(ChatEntry::user(query.clone()));
        }

        self.messages.push(ChatEntry::assistant(String::new()));
        self.pending_answer = Some(self.messages.len() - 1);
        self.last_query = Some(query.clone());
        self.socket_error = None;
        self.tool_label = None;
        self.status = ChatStatus::Thinking;
        self.pulse = true;

        let stream = cosmic::iced::stream::channel(
            16,
            move |mut sender: futures::channel::mpsc::Sender<SocketEvent>| async move {
                if let Err(error) = socket::request_stream(query, &mut sender).await {
                    let _ = sender.send(SocketEvent::Failed(error)).await;
                }
            },
        );

        Task::run(stream, |event| cosmic::Action::App(Message::Socket(event)))
    }

    fn begin_sense(
        &mut self,
        query: String,
        label: String,
        request: Value,
        add_user_message: bool,
    ) -> Task<cosmic::Action<Message>> {
        if add_user_message {
            self.messages.push(ChatEntry::user(query.clone()));
        }
        self.messages.push(ChatEntry::tool(label));
        self.messages.push(ChatEntry::assistant(String::new()));
        self.pending_answer = Some(self.messages.len() - 1);
        self.last_query = Some(query);
        self.socket_error = None;
        self.tool_label = None;
        self.status = ChatStatus::Thinking;
        self.pulse = true;

        let stream = cosmic::iced::stream::channel(
            16,
            move |mut sender: futures::channel::mpsc::Sender<SocketEvent>| async move {
                if let Err(error) = socket::request_value(request, &mut sender).await {
                    let _ = sender.send(SocketEvent::Failed(error)).await;
                }
            },
        );

        Task::run(stream, |event| cosmic::Action::App(Message::Socket(event)))
    }

    fn begin_authorize(
        &mut self,
        request_id: String,
        approved: bool,
        form: Option<Value>,
    ) -> Task<cosmic::Action<Message>> {
        self.pending_auth = None;
        self.status = ChatStatus::Thinking;
        self.socket_error = None;
        self.tool_label = None;
        self.messages.push(ChatEntry::tool(if approved {
            "✅ Autorización enviada. Ejecutando en sandbox…".to_owned()
        } else {
            "🛑 Autorización cancelada por el usuario.".to_owned()
        }));
        self.messages.push(ChatEntry::assistant(String::new()));
        self.pending_answer = Some(self.messages.len() - 1);

        let stream = cosmic::iced::stream::channel(
            16,
            move |mut sender: futures::channel::mpsc::Sender<SocketEvent>| async move {
                if let Err(error) =
                    socket::authorize_stream(request_id, approved, form, &mut sender).await
                {
                    let _ = sender.send(SocketEvent::Failed(error)).await;
                }
            },
        );

        Task::run(stream, |event| cosmic::Action::App(Message::Socket(event)))
    }
}

/// Maps `/imagen`, `/voz`, `/escuchar` and `/hablar` to daemon requests.
fn sense_request(query: &str) -> Option<(String, Value)> {
    let query = query.trim();

    if let Some(rest) = query.strip_prefix("/imagen") {
        let rest = rest.trim();
        let mut parts = rest.splitn(2, char::is_whitespace);
        let path = parts.next().unwrap_or("").trim();
        let prompt = parts
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let label = if path.is_empty() {
            "👁️ Falta la ruta de la imagen.".to_owned()
        } else {
            format!("👁️ Analizando {path}…")
        };
        return Some((
            label,
            json!({ "request": "describe_image", "path": path, "prompt": prompt }),
        ));
    }

    if let Some(rest) = query.strip_prefix("/voz") {
        let path = rest.trim();
        let label = if path.is_empty() {
            "🎤 Falta la ruta del audio.".to_owned()
        } else {
            format!("🎤 Transcribiendo {path}…")
        };
        return Some((label, json!({ "request": "transcribe", "path": path })));
    }

    if let Some(rest) = query.strip_prefix("/escuchar") {
        let seconds = rest.trim().parse::<u32>().unwrap_or(5).clamp(1, 60);
        return Some((
            format!("🎙️ Grabando {seconds} segundos…"),
            json!({ "request": "listen", "seconds": seconds }),
        ));
    }

    if let Some(rest) = query.strip_prefix("/hablar") {
        let text = rest.trim();
        return Some((
            "🔊 Generando voz con Piper…".to_owned(),
            json!({ "request": "speak", "text": text }),
        ));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{sense_request, AuthField, AuthPrompt};
    use serde_json::json;

    fn prompt(mode: &str, fields: Vec<AuthField>) -> AuthPrompt {
        AuthPrompt {
            mode: mode.to_owned(),
            request_id: "AUTH-1".to_owned(),
            question: "¿Seguro?".to_owned(),
            proposed_action: String::new(),
            authorization_code: Some("123456".to_owned()),
            fields,
            error: None,
        }
    }

    fn field(name: &str, value: &str) -> AuthField {
        AuthField {
            name: name.to_owned(),
            label: name.to_owned(),
            required: true,
            value: value.to_owned(),
        }
    }

    #[test]
    fn secure_prompt_rejects_empty_fields_and_wrong_codes() {
        let empty = prompt("secure", vec![field("target", "")]);
        assert!(empty.validate().is_err());

        let wrong = prompt("secure", vec![field("authorization_code", "000000")]);
        assert!(wrong.validate().is_err());

        let valid = prompt("secure", vec![field("authorization_code", "123456")]);
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn simple_prompt_needs_no_fields() {
        let simple = prompt("simple", Vec::new());
        assert!(simple.validate().is_ok());
    }

    #[test]
    fn prompt_serializes_its_values_as_a_form() {
        let form = prompt("secure", vec![field("target", " 10.0.0.1 ")]);
        assert_eq!(form.form_value(), json!({ "target": "10.0.0.1" }));
    }

    #[test]
    fn parses_sense_commands() {
        let (_, request) = sense_request("/imagen /tmp/a.png describe esto").unwrap();
        assert_eq!(request["request"], "describe_image");
        assert_eq!(request["path"], "/tmp/a.png");
        assert_eq!(request["prompt"], "describe esto");

        let (_, request) = sense_request("/escuchar 7").unwrap();
        assert_eq!(request["request"], "listen");
        assert_eq!(request["seconds"], 7);

        let (_, request) = sense_request("/hablar hola mundo").unwrap();
        assert_eq!(request["request"], "speak");
        assert_eq!(request["text"], "hola mundo");

        assert!(sense_request("hola").is_none());
    }
}
