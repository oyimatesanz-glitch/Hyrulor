pub mod theme;

use cosmic::iced::{widget::Space, Alignment, Length};
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

#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    input_value: &'a str,
    entries: &'a [ChatEntry],
    status: ChatStatus,
    tool_label: Option<&'a str>,
    socket_error: Option<&'a str>,
    can_retry: bool,
    auth: Option<&'a crate::app::AuthPrompt>,
    secure_mode: Option<bool>,
) -> Element<'a, crate::app::Message> {
    let mode_label = match secure_mode {
        Some(true) => "🔐 SEGURO",
        Some(false) => "🔓 SIMPLE",
        None => "…",
    };
    let mode_button = cosmic::iced::widget::button(widget::text(mode_label).size(10))
        .on_press(crate::app::Message::ToggleMode)
        .padding([4, 6]);

    let header = widget::row![
        widget::text("HYRULOR").size(20),
        Space::new().width(Length::Fill),
        widget::text("●").size(10),
        mode_button,
    ]
    .align_y(Alignment::Center)
    .spacing(8);

    let mut conversation = widget::column![].spacing(12).width(Length::Fill);

    if entries.is_empty() {
        conversation = conversation.push(
            widget::container(
                widget::column![
                    widget::text("A su servicio.").size(18),
                    widget::text("¿En qué puedo asistirle?").size(14),
                    widget::text("La conexión local se establece al enviar su primera consulta.")
                        .size(11),
                    widget::text("Comandos: /imagen, /voz, /escuchar, /hablar").size(10),
                ]
                .spacing(7),
            )
            .padding(18)
            .width(Length::Fill)
            .style(theme::container_style(theme::card())),
        );
    }

    let quick_actions = widget::row![
        cosmic::iced::widget::button(widget::text("BATERÍA").size(10))
            .on_press(crate::app::Message::QuickQuery(
                "¿Cuál es el estado de la batería?".to_owned()
            ))
            .padding([6, 8]),
        cosmic::iced::widget::button(widget::text("RED").size(10))
            .on_press(crate::app::Message::QuickQuery(
                "Realice un diagnóstico de la conexión de red.".to_owned()
            ))
            .padding([6, 8]),
        cosmic::iced::widget::button(widget::text("SISTEMA").size(10))
            .on_press(crate::app::Message::QuickQuery(
                "Resuma el estado general del sistema.".to_owned()
            ))
            .padding([6, 8]),
    ]
    .spacing(6)
    .width(Length::Fill);

    for entry in entries {
        let (label, bg, border, align_right) = match entry.role {
            ChatRole::Assistant => ("HYRULOR", theme::assistant_bubble(), theme::brass(), false),
            ChatRole::User => ("USTED", theme::user_bubble(), theme::sage(), true),
            ChatRole::Tool => (
                "REGISTRO DE HERRAMIENTA",
                theme::tool_bubble(),
                theme::brass_dim(),
                false,
            ),
        };

        let bubble_content = widget::column![
            widget::text(label).size(10),
            widget::text(entry.body.as_str()).size(if entry.role == ChatRole::Tool {
                12
            } else {
                14
            }),
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
                .push(widget::row![Space::new().width(Length::Fill), bubble].width(Length::Fill));
        } else {
            conversation = conversation
                .push(widget::row![bubble, Space::new().width(Length::Fill)].width(Length::Fill));
        }
    }

    let conversation_scroll = widget::scrollable(conversation)
        .height(Length::Fill)
        .width(Length::Fill)
        .anchor_bottom();

    let status_text = if auth.is_some() {
        "Revise y confirme la acción antes de continuar."
    } else {
        match status {
            ChatStatus::Idle => "",
            ChatStatus::Thinking => "Razonando…",
            ChatStatus::Tool => tool_label.unwrap_or("Consultando herramientas…"),
            ChatStatus::Error => "El daemon de Hyrulor no responde.",
        }
    };

    let status_line = widget::text(status_text).size(12);

    let input = widget::text_input("Escriba su consulta…", input_value)
        .on_input(crate::app::Message::InputChanged)
        .on_submit(|_| crate::app::Message::Send)
        .size(14)
        .padding([12, 10])
        .width(Length::Fill);

    let send_button = cosmic::iced::widget::button(widget::text("↑").size(18))
        .on_press(crate::app::Message::Send)
        .padding([8, 13]);

    let composer = widget::row![input, send_button]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let mut footer = widget::column![status_line].spacing(8).width(Length::Fill);
    if auth.is_none() {
        footer = footer.push(composer);
    }

    if let Some(error) = socket_error {
        footer = footer.push(
            widget::row![
                widget::text(error).size(10),
                Space::new().width(Length::Fill),
                cosmic::iced::widget::button(widget::text("Reintentar").size(12))
                    .on_press(crate::app::Message::Retry)
                    .padding([6, 10]),
            ]
            .align_y(Alignment::Center),
        );
    } else if can_retry && status == ChatStatus::Error {
        footer = footer.push(
            cosmic::iced::widget::button(widget::text("Reintentar"))
                .on_press(crate::app::Message::Retry),
        );
    }

    let main_area: Element<'a, crate::app::Message> = match auth {
        Some(prompt) => auth_panel(prompt),
        None => conversation_scroll.into(),
    };

    let mut body = widget::column![widget::container(header)
        .padding([4, 10])
        .width(Length::Fill)]
    .spacing(4)
    .width(Length::Fill)
    .height(Length::Fill);

    if auth.is_none() {
        body = body
            .push(
                widget::container(quick_actions)
                    .padding([4, 10])
                    .width(Length::Fill),
            )
            .push(
                widget::container(widget::text("────────────────────────────────────────").size(8))
                    .padding([0, 10]),
            );
    }

    body = body
        .push(
            widget::container(main_area)
                .padding([10, 12])
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .push(
            widget::container(footer)
                .padding([12, 12])
                .width(Length::Fill),
        );

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

/// Secure mode: full authorization form. Simple mode: yes/no confirmation.
fn auth_panel<'a>(auth: &'a crate::app::AuthPrompt) -> Element<'a, crate::app::Message> {
    let secure = auth.mode == "secure";
    let title = if secure {
        "AUTORIZACIÓN DE ACCIÓN OFENSIVA"
    } else {
        "CONFIRMACIÓN REQUERIDA"
    };

    let mut content = widget::column![
        widget::text(title).size(13),
        widget::text(if secure {
            "Modo seguro · formulario completo"
        } else {
            "Modo simple · confirmación y/n"
        })
        .size(10),
        widget::text(auth.question.as_str()).size(14),
    ]
    .spacing(8)
    .width(Length::Fill);

    if !auth.proposed_action.is_empty() {
        content = content.push(
            widget::container(
                widget::column![
                    widget::text("ACCIÓN PROPUESTA").size(9),
                    widget::text(auth.proposed_action.as_str()).size(11),
                ]
                .spacing(4),
            )
            .padding(10)
            .width(Length::Fill)
            .style(theme::container_style(theme::tool_bubble())),
        );
    }

    if secure {
        if let Some(code) = &auth.authorization_code {
            content = content.push(
                widget::container(
                    widget::row![
                        widget::text("CÓDIGO").size(10),
                        widget::text(code.as_str()).size(16),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .padding([8, 10])
                .width(Length::Fill)
                .style(theme::container_style(theme::card())),
            );
        }

        let mut fields = widget::column![].spacing(8);
        for (index, field) in auth.fields.iter().enumerate() {
            let input = widget::text_input("", field.value.as_str())
                .on_input(move |value| crate::app::Message::AuthFieldChanged(index, value))
                .size(12)
                .padding([8, 8])
                .width(Length::Fill);
            fields = fields.push(
                widget::column![
                    widget::text(format!(
                        "{}{}",
                        field.label,
                        if field.required { " *" } else { "" }
                    ))
                    .size(10),
                    input,
                ]
                .spacing(3),
            );
        }
        content = content.push(fields);
    }

    if let Some(error) = &auth.error {
        content = content.push(widget::text(error.as_str()).size(11));
    }

    let cancel = cosmic::iced::widget::button(widget::text("CANCELAR").size(11))
        .on_press(crate::app::Message::AuthCancel)
        .padding([8, 12]);
    let approve_label = if secure { "AUTORIZAR" } else { "SÍ (y)" };
    let approve = cosmic::iced::widget::button(widget::text(approve_label).size(11))
        .on_press(crate::app::Message::AuthSubmit)
        .padding([8, 12]);

    content = content.push(
        widget::row![cancel, Space::new().width(Length::Fill), approve]
            .align_y(Alignment::Center)
            .width(Length::Fill),
    );

    widget::scrollable(
        widget::container(content)
            .padding(14)
            .width(Length::Fill)
            .style(theme::container_style(theme::card())),
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .into()
}
