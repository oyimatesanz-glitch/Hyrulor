use std::time::Duration;

use cosmic::iced::futures::{self, SinkExt};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{time, window::Id, Limits, Subscription};
use cosmic::prelude::*;
use cosmic::widget;

use crate::socket::{self, SocketEvent};
use crate::ui::{self, ChatEntry, ChatStatus};

const APP_ID: &str = "com.github.oyimatesanz-glitch.hyrulor-applet";
const POPUP_WIDTH: f32 = 420.0;
const POPUP_HEIGHT: f32 = 620.0;

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
        (
            Self {
                core,
                ..Self::default()
            },
            Task::none(),
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
                if self.status != ChatStatus::Thinking && self.status != ChatStatus::Tool {
                    self.input.clear();
                    return self.begin_request(query, true);
                }
            }
            Message::Send => {
                let query = self.input.trim().to_owned();
                if !query.is_empty()
                    && self.status != ChatStatus::Thinking
                    && self.status != ChatStatus::Tool
                {
                    self.input.clear();
                    return self.begin_request(query, true);
                }
            }
            Message::Retry => {
                if let Some(query) = self.last_query.clone() {
                    return self.begin_request(query, false);
                }
            }
            Message::Pulse => {
                self.pulse = !self.pulse;
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
}
