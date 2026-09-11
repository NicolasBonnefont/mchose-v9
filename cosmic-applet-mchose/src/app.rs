//! A camada de UI: traduz evento em `Message` e estado em widget.
//!
//! Ela nao decide nada — quem decide o que mostrar e o [`crate::state`], que nao
//! conhece libcosmic. Aqui so ha traducao.

use std::time::Duration;

use cosmic::app::{Core, Task};
use cosmic::iced::window::Id;
use cosmic::iced::{Rectangle, Subscription, stream};
use cosmic::surface::action::{app_popup, destroy_popup};
use cosmic::widget;
use cosmic::{Element, iced::core::window};
use futures::{SinkExt, StreamExt};
use mchose_device::{Demand, DeviceEvent};

use crate::state::AppletState;

const ID: &str = "com.github.NicolasBonnefont.mchose-v9.Applet";
/// Espera antes de tentar de novo quando o fluxo cai.
const RETENTATIVA: Duration = Duration::from_secs(5);

pub struct Window {
    core: Core,
    popup: Option<Id>,
    state: AppletState,
    demand: Option<Demand>,
}

#[derive(Clone, Debug)]
pub enum Message {
    PopupClosed(Id),
    Surface(cosmic::surface::Action),
    /// O fluxo abriu e entregou a alca de consulta.
    Ready(Demand),
    Device(DeviceEvent),
    /// O fluxo caiu; ha nova tentativa a caminho.
    StreamFailed(String),
}

impl cosmic::Application for Window {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Message>) {
        (
            Self {
                core,
                popup: None,
                state: AppletState::default(),
                demand: None,
            },
            Task::none(),
        )
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn subscription(&self) -> Subscription<Message> {
        // O identificador e constante: derivar de estado recriaria a thread, o fd
        // do hidraw e o socket do monitor a cada mudanca.
        Subscription::run_with(0u8, |_| {
            stream::channel(50, async |mut output| {
                // O laco nunca sai. Fluxo que acaba vira estado visivel e nova
                // tentativa — fim de assinatura seria applet mudo ate o painel
                // reiniciar.
                loop {
                    match mchose_device::events() {
                        Ok((fluxo, demand)) => {
                            if output.send(Message::Ready(demand)).await.is_err() {
                                return;
                            }
                            futures::pin_mut!(fluxo);
                            while let Some(evento) = fluxo.next().await {
                                if output.send(Message::Device(evento)).await.is_err() {
                                    return;
                                }
                            }
                            let _ = output
                                .send(Message::StreamFailed("fluxo encerrado".to_owned()))
                                .await;
                        }
                        Err(e) => {
                            let _ = output.send(Message::StreamFailed(e.to_string())).await;
                        }
                    }
                    tokio::time::sleep(RETENTATIVA).await;
                }
            })
        })
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PopupClosed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                }
            }
            Message::Surface(a) => {
                return cosmic::task::message(cosmic::Action::Cosmic(
                    cosmic::app::Action::Surface(a),
                ));
            }
            Message::Ready(demand) => {
                // Pede uma leitura assim que o fluxo abre: se o fone estiver
                // mudo, o push sozinho pode nunca vir.
                demand.refresh();
                self.demand = Some(demand);
            }
            Message::Device(evento) => self.state.apply(evento),
            Message::StreamFailed(motivo) => {
                self.demand = None;
                self.state.stream_failed(&motivo);
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let texto = self
            .state
            .panel_percent()
            .map_or_else(|| "—".to_owned(), |p| format!("{p}%"));

        let conteudo = widget::row::with_children(vec![
            widget::icon::from_name("audio-headphones-symbolic")
                .symbolic(true)
                .size(14)
                .into(),
            self.core.applet.text(texto).into(),
        ])
        .spacing(4)
        .align_y(cosmic::iced::Alignment::Center);

        let aberto = self.popup;
        widget::button::custom(conteudo)
            .class(cosmic::theme::Button::AppletIcon)
            .on_press_with_rectangle(move |offset, bounds| {
                if let Some(id) = aberto {
                    Message::Surface(destroy_popup(id))
                } else {
                    Message::Surface(app_popup::<Window>(
                        |_| Default::default(),
                        move |state: &mut Window| {
                            // Abrir o popover e o gatilho sob demanda que o card
                            // #2 construiu. Sem dongle e no-op.
                            if let Some(d) = state.demand.as_ref() {
                                d.refresh();
                            }
                            let novo = Id::unique();
                            state.popup = Some(novo);
                            let Some(principal) = state.core.main_window_id() else {
                                return state.core.applet.get_popup_settings(
                                    Id::NONE,
                                    novo,
                                    None,
                                    None,
                                    None,
                                );
                            };
                            let mut settings = state
                                .core
                                .applet
                                .get_popup_settings(principal, novo, None, None, None);
                            settings.positioner.anchor_rect = Rectangle {
                                x: (bounds.x - offset.x) as i32,
                                y: (bounds.y - offset.y) as i32,
                                width: bounds.width as i32,
                                height: bounds.height as i32,
                            };
                            settings
                        },
                        Some(Box::new(move |state: &Window| {
                            Element::from(state.core.applet.popup_container(state.popover()))
                                .map(cosmic::Action::App)
                        })),
                    ))
                }
            })
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Message> {
        self.popover()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

impl Window {
    /// O conteudo do popover. Sem byte cru, sem caminho de device.
    fn popover(&self) -> Element<'_, Message> {
        let percentual = self
            .state
            .panel_percent()
            .map_or_else(|| "sem leitura".to_owned(), |p| format!("{p}%"));

        widget::column::with_children(vec![
            widget::text::title4("MCHOSE V9 PRO").into(),
            widget::text::body(percentual).into(),
            widget::text::caption(self.state.status().to_owned()).into(),
            widget::divider::horizontal::default().into(),
            widget::text::caption(format!(
                "firmware do dongle: {}",
                self.state.dongle_firmware()
            ))
            .into(),
            widget::text::caption(format!(
                "firmware do fone: {}",
                self.state.headset_firmware()
            ))
            .into(),
        ])
        .spacing(6)
        .padding(12)
        .into()
    }
}
