pub mod theme;

use cosmic::iced::{Alignment, Background, Length};
use cosmic::prelude::*;
use cosmic::widget;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChatStatus {
    #[default]
    Idle,
    Thinking,
    Tool,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    Assistant,
    User,
    Tool,
}

#[derive(Debug, Clone)]
pub struct ChatEntry {
    pub role: ChatRole,
    pub body: String,
}

impl ChatEntry {
    pub fn user(body: String) -> Self {
        Self {
            role: ChatRole::User,
            body,
        }
    }

    pub fn assistant(body: String) -> Self {
        Self {
            role: ChatRole::Assistant,
            body,
        }
    }

    pub fn tool(body: String) -> Self {
        Self {
            role: ChatRole::Tool,
            body,
        }
    }
}

pub fn view<'a>(
    input_value: &'a str,
    entries: &'a [ChatEntry],
    status: ChatStatus,
    tool_label: Option<&'a str>,
    socket_error: Option<&'a str>,
    can_retry: bool,
) -> Element<'a, crate::app::Message> {
    let header = widget::row![
        widget::text("HYRULOR").size(20).color(theme::champagne()),
        Space::with_width(Length::Fill),
        widget::text("●").size(10).color(theme::sage()),
        widget::text("DIGITAL ARISTOCRACY")
            .size(10)
            .color(theme::muted()),
    ]
    .align_y(Alignment::Center)
    .spacing(8);

    let mut conversation = widget::column![].spacing(12).width(Length::Fill);

    if entries.is_empty() {
        conversation = conversation.push(
            widget::container(
                widget::column![
                    widget::text("A su servicio.")
                        .size(18)
                        .color(theme::champagne()),
                    widget::text("¿En qué puedo asistirle?")
                        .size(14)
                        .color(theme::cream()),
                    widget::text("La conexión local se establece al enviar su primera consulta.")
                        .size(11)
                        .color(theme::muted()),
                ]
                .spacing(7),
            )
            .padding(18)
            .width(Length::Fill)
            .style(theme::container_style(theme::card())),
        );
    }

    let quick_actions = widget::row![
        cosmic::iced::widget::button(widget::text("BATERÍA").size(10).color(theme::cream()))
            .on_press(crate::app::Message::QuickQuery(
                "¿Cuál es el estado de la batería?".to_owned()
            ))
            .padding([6, 8])
            .style(|_, _| cosmic::iced::widget::button::Style {
                background: Some(Background::Color(theme::card())),
                text_color: theme::cream(),
                ..Default::default()
            }),
        cosmic::iced::widget::button(widget::text("RED").size(10).color(theme::cream()))
            .on_press(crate::app::Message::QuickQuery(
                "Realice un diagnóstico de la conexión de red.".to_owned()
            ))
            .padding([6, 8])
            .style(|_, _| cosmic::iced::widget::button::Style {
                background: Some(Background::Color(theme::card())),
                text_color: theme::cream(),
                ..Default::default()
            }),
        cosmic::iced::widget::button(widget::text("SISTEMA").size(10).color(theme::cream()))
            .on_press(crate::app::Message::QuickQuery(
                "Resuma el estado general del sistema.".to_owned()
            ))
            .padding([6, 8])
            .style(|_, _| cosmic::iced::widget::button::Style {
                background: Some(Background::Color(theme::card())),
                text_color: theme::cream(),
                ..Default::default()
            }),
    ]
    .spacing(6)
    .width(Length::Fill);

    for entry in entries {
        let (label, bg, fg, border, align_right) = match entry.role {
            ChatRole::Assistant => (
                "HYRULOR",
                theme::assistant_bubble(),
                theme::cream(),
                theme::brass(),
                false,
            ),
            ChatRole::User => (
                "USTED",
                theme::user_bubble(),
                theme::cream(),
                theme::sage(),
                true,
            ),
            ChatRole::Tool => (
                "REGISTRO DE HERRAMIENTA",
                theme::tool_bubble(),
                theme::champagne(),
                theme::brass_dim(),
                false,
            ),
        };

        let bubble_content = widget::column![
            widget::text(label).size(10).color(theme::brass()),
            widget::text(entry.body.as_str())
                .size(if entry.role == ChatRole::Tool { 12 } else { 14 })
                .color(fg),
        ]
        .spacing(5);

        // The outer one-pixel brass layer acts as a restrained frame. The inner
        // surface stays square, in keeping with the design brief.
        let bubble = widget::container(
            widget::container(bubble_content)
                .padding([10, 12])
                .width(Length::Fill)
                .style(theme::container_style(bg)),
        )
        .padding(1)
        .max_width(372.0)
        .style(theme::container_style(border));

        if align_right {
            conversation = conversation
                .push(widget::row![Space::with_width(Length::Fill), bubble].width(Length::Fill));
        } else {
            conversation = conversation
                .push(widget::row![bubble, Space::with_width(Length::Fill)].width(Length::Fill));
        }
    }

    let conversation_scroll = widget::scrollable(conversation)
        .height(Length::Fill)
        .width(Length::Fill)
        .anchor_bottom();

    let status_text = match status {
        ChatStatus::Idle => "",
        ChatStatus::Thinking => "Razonando…",
        ChatStatus::Tool => tool_label.unwrap_or("Consultando herramientas…"),
        ChatStatus::Error => "El daemon de Hyrulor no responde.",
    };

    let status_line = widget::text(status_text)
        .size(12)
        .color(if status == ChatStatus::Error {
            theme::oxblood_light()
        } else {
            theme::brass()
        });

    let input = widget::text_input("Escriba su consulta…", input_value)
        .on_input(crate::app::Message::InputChanged)
        .on_submit(|_| crate::app::Message::Send)
        .size(14)
        .padding([12, 10])
        .width(Length::Fill);

    let send_button =
        cosmic::iced::widget::button(widget::text("↑").size(18).color(theme::forest()))
            .on_press(crate::app::Message::Send)
            .padding([8, 13])
            .style(|_, status| {
                let background = match status {
                    cosmic::iced::widget::button::Status::Hovered => theme::champagne(),
                    _ => theme::brass(),
                };
                cosmic::iced::widget::button::Style {
                    background: Some(Background::Color(background)),
                    text_color: theme::forest(),
                    ..Default::default()
                }
            });

    let composer = widget::row![input, send_button]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let mut footer = widget::column![status_line, composer]
        .spacing(8)
        .width(Length::Fill);

    if let Some(error) = socket_error {
        footer = footer.push(
            widget::row![
                widget::text(error).size(10).color(theme::muted()),
                Space::with_width(Length::Fill),
                cosmic::iced::widget::button(
                    widget::text("Reintentar").size(12).color(theme::forest())
                )
                .on_press(crate::app::Message::Retry)
                .padding([6, 10])
                .style(|_, _| cosmic::iced::widget::button::Style {
                    background: Some(Background::Color(theme::brass())),
                    text_color: theme::forest(),
                    ..Default::default()
                }),
            ]
            .align_y(Alignment::Center),
        );
    } else if can_retry && status == ChatStatus::Error {
        footer = footer.push(widget::button("Reintentar").on_press(crate::app::Message::Retry));
    }

    let body = widget::column![
        widget::container(header)
            .padding([4, 10])
            .width(Length::Fill),
        widget::container(quick_actions)
            .padding([4, 10])
            .width(Length::Fill),
        widget::container(
            widget::text("────────────────────────────────────────")
                .size(8)
                .color(theme::brass_dim())
        )
        .padding([0, 10]),
        widget::container(conversation_scroll)
            .padding([10, 12])
            .width(Length::Fill)
            .height(Length::Fill),
        widget::container(footer)
            .padding([12, 12])
            .width(Length::Fill),
    ]
    .spacing(4)
    .width(Length::Fill)
    .height(Length::Fill);

    let body_surface = widget::container(body)
        .padding(2)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::container_style(theme::forest()));

    let inner_frame = widget::container(body_surface)
        .padding(1)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::container_style(theme::cream()));

    widget::container(inner_frame)
        .padding(1)
        .width(420.0)
        .height(620.0)
        .style(theme::container_style(theme::brass()))
        .into()
}
