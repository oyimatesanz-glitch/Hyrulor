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
    pulse: bool,
) -> Element<'a, crate::app::Message> {
    let mode_label = match secure_mode {
        Some(true) => "🔐 SEGURO",
        Some(false) => "🔓 SIMPLE",
        None => "…",
    };
    let mode_button = cosmic::iced::widget::button(widget::text(mode_label).size(10))
        .on_press(crate::app::Message::ToggleMode)
        .padding([7, 13])
        .class(theme::outline_button());

    let (status_color, status_label) = if auth.is_some() {
        (theme::gold(), "AUTORIZACIÓN PENDIENTE")
    } else {
        match status {
            ChatStatus::Idle => (theme::success(), "LOCAL · LISTO"),
            ChatStatus::Thinking => (
                if pulse {
                    theme::gold_light()
                } else {
                    theme::gold()
                },
                "RAZONANDO",
            ),
            ChatStatus::Tool => (theme::cyan(), "HERRAMIENTA"),
            ChatStatus::Error => (theme::danger(), "SIN CONEXIÓN"),
        }
    };

    let avatar = widget::container(
        widget::text("H")
            .size(16)
            .class(theme::text_color(theme::background())),
    )
    .width(40)
    .height(40)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .style(theme::surface_framed(
        theme::gold(),
        theme::gold_light(),
        20.0,
    ));

    let title = widget::column![
        widget::text("HYRULOR")
            .size(15)
            .class(theme::text_color(theme::gold())),
        widget::row![
            widget::text("●")
                .size(8)
                .class(theme::text_color(status_color)),
            widget::text(status_label)
                .size(10)
                .class(theme::text_color(theme::text_muted())),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    ]
    .spacing(3);

    let clear_button = cosmic::iced::widget::button(widget::text("LIMPIAR").size(10))
        .on_press(crate::app::Message::Clear)
        .padding([7, 11])
        .class(theme::ghost_button());

    let header = widget::row![
        avatar,
        title,
        Space::new().width(Length::Fill),
        clear_button,
        mode_button,
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut conversation = widget::column![].spacing(14).width(Length::Fill);

    if entries.is_empty() {
        conversation = conversation.push(
            widget::container(
                widget::column![
                    widget::text("A su servicio.")
                        .size(19)
                        .class(theme::text_color(theme::gold())),
                    widget::text("¿En qué puedo asistirle?")
                        .size(14)
                        .class(theme::text_color(theme::text_primary())),
                    widget::text(
                        "Conexión local cifrada · sin nube · cada acción sensible requiere su autorización."
                    )
                    .size(11)
                    .class(theme::text_color(theme::text_secondary())),
                    widget::text("Comandos: /imagen · /voz · /escuchar · /hablar")
                        .size(10)
                        .class(theme::text_color(theme::cyan())),
                ]
                .spacing(8),
            )
            .padding(20)
            .width(Length::Fill)
            .style(theme::surface_radius(theme::card(), 20.0)),
        );
    }

    let quick_actions = widget::row![
        cosmic::iced::widget::button(widget::text("BATERÍA").size(10))
            .on_press(crate::app::Message::QuickQuery(
                "¿Cuál es el estado de la batería?".to_owned()
            ))
            .padding([7, 12])
            .class(theme::outline_button()),
        cosmic::iced::widget::button(widget::text("RED").size(10))
            .on_press(crate::app::Message::QuickQuery(
                "Realice un diagnóstico de la conexión de red.".to_owned()
            ))
            .padding([7, 12])
            .class(theme::outline_button()),
        cosmic::iced::widget::button(widget::text("SISTEMA").size(10))
            .on_press(crate::app::Message::QuickQuery(
                "Resuma el estado general del sistema.".to_owned()
            ))
            .padding([7, 12])
            .class(theme::outline_button()),
    ]
    .spacing(8)
    .width(Length::Fill);

    for entry in entries {
        let (label, bg, border, label_color, align_right) = match entry.role {
            ChatRole::Assistant => (
                "HYRULOR",
                theme::assistant_bubble(),
                theme::border_subtle(),
                theme::gold(),
                false,
            ),
            ChatRole::User => (
                "USTED",
                theme::user_bubble(),
                theme::border_gold(),
                theme::gold_light(),
                true,
            ),
            ChatRole::Tool => (
                "REGISTRO DE HERRAMIENTA",
                theme::tool_bubble(),
                theme::border_cyan(),
                theme::cyan(),
                false,
            ),
        };

        let bubble_content = widget::column![
            widget::text(label)
                .size(9)
                .class(theme::text_color(label_color)),
            widget::text(entry.body.as_str())
                .size(if entry.role == ChatRole::Tool { 12 } else { 14 })
                .class(theme::text_color(theme::text_primary())),
        ]
        .spacing(6);

        let bubble = widget::container(bubble_content)
            .padding([12, 16])
            .max_width(348.0)
            .style(theme::surface_framed(bg, border, 20.0));

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

    let status_line = if auth.is_some() {
        (
            "Revise y confirme la acción antes de continuar.",
            theme::gold(),
        )
    } else {
        match status {
            ChatStatus::Idle => ("", theme::text_muted()),
            ChatStatus::Thinking => ("Razonando…", theme::gold()),
            ChatStatus::Tool => (
                tool_label.unwrap_or("Consultando herramientas…"),
                theme::cyan(),
            ),
            ChatStatus::Error => ("El daemon de Hyrulor no responde.", theme::danger()),
        }
    };

    let input = widget::text_input("Escriba su consulta…", input_value)
        .on_input(crate::app::Message::InputChanged)
        .on_submit(|_| crate::app::Message::Send)
        .size(13)
        .padding([13, 18])
        .width(Length::Fill);

    let send_button = cosmic::iced::widget::button(widget::text("↑").size(18))
        .on_press(crate::app::Message::Send)
        .padding([9, 14])
        .class(theme::primary_button());

    let composer = widget::row![input, send_button]
        .spacing(10)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let mut footer = widget::column![widget::text(status_line.0)
        .size(11)
        .class(theme::text_color(status_line.1))]
    .spacing(10)
    .width(Length::Fill);

    if auth.is_none() {
        footer = footer.push(composer);
    }

    if let Some(error) = socket_error {
        footer = footer.push(
            widget::row![
                widget::text(error)
                    .size(10)
                    .class(theme::text_color(theme::danger())),
                Space::new().width(Length::Fill),
                cosmic::iced::widget::button(widget::text("REINTENTAR").size(10))
                    .on_press(crate::app::Message::Retry)
                    .padding([6, 11])
                    .class(theme::outline_button()),
            ]
            .align_y(Alignment::Center),
        );
    } else if can_retry && status == ChatStatus::Error {
        footer = footer.push(
            cosmic::iced::widget::button(widget::text("REINTENTAR").size(11))
                .on_press(crate::app::Message::Retry)
                .padding([7, 12])
                .class(theme::outline_button()),
        );
    }

    let main_area: Element<'a, crate::app::Message> = match auth {
        Some(prompt) => auth_panel(prompt),
        None => conversation_scroll.into(),
    };

    let divider = || {
        widget::container(Space::new().height(1))
            .width(Length::Fill)
            .style(theme::surface_radius(theme::border_subtle(), 0.0))
    };

    let mut body = widget::column![]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

    body = body.push(
        widget::container(header)
            .padding([16, 20])
            .width(Length::Fill),
    );

    if auth.is_none() {
        body = body.push(divider()).push(
            widget::container(quick_actions)
                .padding([10, 20])
                .width(Length::Fill),
        );
    }

    body = body
        .push(divider())
        .push(
            widget::container(main_area)
                .padding([16, 18])
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .push(
            widget::container(footer)
                .padding([14, 18])
                .width(Length::Fill)
                .style(theme::surface_radius(theme::glass(), 0.0)),
        );

    widget::container(body)
        .width(420.0)
        .height(620.0)
        .style(theme::surface_framed(
            theme::glass(),
            theme::border_subtle(),
            28.0,
        ))
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
        widget::text(title)
            .size(13)
            .class(theme::text_color(theme::gold())),
        widget::text(if secure {
            "Modo seguro · formulario completo"
        } else {
            "Modo simple · confirmación y/n"
        })
        .size(10)
        .class(theme::text_color(theme::text_muted())),
        widget::text(auth.question.as_str())
            .size(14)
            .class(theme::text_color(theme::text_primary())),
    ]
    .spacing(9)
    .width(Length::Fill);

    if !auth.proposed_action.is_empty() {
        content = content.push(
            widget::container(
                widget::column![
                    widget::text("ACCIÓN PROPUESTA")
                        .size(9)
                        .class(theme::text_color(theme::cyan())),
                    widget::text(auth.proposed_action.as_str())
                        .size(11)
                        .class(theme::text_color(theme::text_secondary())),
                ]
                .spacing(5),
            )
            .padding(12)
            .width(Length::Fill)
            .style(theme::surface_radius(theme::tool_bubble(), 14.0)),
        );
    }

    if secure {
        if let Some(code) = &auth.authorization_code {
            content = content.push(
                widget::container(
                    widget::row![
                        widget::text("CÓDIGO")
                            .size(10)
                            .class(theme::text_color(theme::text_muted())),
                        Space::new().width(Length::Fill),
                        widget::text(code.as_str())
                            .size(16)
                            .class(theme::text_color(theme::gold())),
                    ]
                    .align_y(Alignment::Center),
                )
                .padding([9, 12])
                .width(Length::Fill)
                .style(theme::surface_radius(theme::card(), 12.0)),
            );
        }

        let mut fields = widget::column![].spacing(9);
        for (index, field) in auth.fields.iter().enumerate() {
            let input = widget::text_input("", field.value.as_str())
                .on_input(move |value| crate::app::Message::AuthFieldChanged(index, value))
                .size(12)
                .padding([9, 10])
                .width(Length::Fill);
            fields = fields.push(
                widget::column![
                    widget::text(format!(
                        "{}{}",
                        field.label,
                        if field.required { " *" } else { "" }
                    ))
                    .size(10)
                    .class(theme::text_color(theme::text_secondary())),
                    input,
                ]
                .spacing(4),
            );
        }
        content = content.push(fields);
    }

    if let Some(error) = &auth.error {
        content = content.push(
            widget::text(error.as_str())
                .size(11)
                .class(theme::text_color(theme::danger())),
        );
    }

    let cancel = cosmic::iced::widget::button(widget::text("CANCELAR").size(11))
        .on_press(crate::app::Message::AuthCancel)
        .padding([9, 14])
        .class(theme::ghost_button());
    let approve_label = if secure { "AUTORIZAR" } else { "SÍ (y)" };
    let approve = cosmic::iced::widget::button(widget::text(approve_label).size(11))
        .on_press(crate::app::Message::AuthSubmit)
        .padding([9, 14])
        .class(theme::primary_button());

    content = content.push(
        widget::row![cancel, Space::new().width(Length::Fill), approve]
            .align_y(Alignment::Center)
            .width(Length::Fill),
    );

    widget::scrollable(
        widget::container(content)
            .padding(18)
            .width(Length::Fill)
            .style(theme::surface_radius(theme::card(), 20.0)),
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .into()
}
