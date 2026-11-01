//! The site form: a new site or an edit of a saved one, drawn as dbdelve
//! draws its connection form — a header with the address, wells of rows.

use gpui::{
    AnyElement, AppContext, Context, Entity, FontWeight, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    Disableable,
    input::{Input, InputEvent, InputState},
};

use crate::{
    icons::{icon, icon as icons},
    remote::{Credentials, Remote, runtime},
    store::{self, Secret, Site},
    theme::{SiteColor, Theme, layout},
    ui::{self, Control, Tone},
};

use super::{Modal, Workspace};

pub struct SiteForm {
    /// The id of the site being edited; `None` for a new one.
    pub editing: Option<String>,
    pub name: Entity<InputState>,
    pub host: Entity<InputState>,
    pub port: Entity<InputState>,
    pub user: Entity<InputState>,
    pub password: Entity<InputState>,
    pub key_path: Entity<InputState>,
    pub passphrase: Entity<InputState>,
    pub remote_path: Entity<InputState>,
    pub local_path: Entity<InputState>,
    pub use_agent: bool,
    pub color: Option<SiteColor>,
    pub error: Option<String>,
    pub testing: bool,
    pub tested: Option<Result<(), String>>,
}

impl SiteForm {
    fn site(&self, cx: &gpui::App, taken: &[String]) -> Result<Site, String> {
        let value = |input: &Entity<InputState>| input.read(cx).value().trim().to_string();
        let optional = |input: &Entity<InputState>| Some(value(input)).filter(|v| !v.is_empty());
        let host = value(&self.host);
        if host.is_empty() {
            return Err("A host is needed.".into());
        }
        let port = match value(&self.port) {
            port if port.is_empty() => 22,
            port => port
                .parse::<u16>()
                .ok()
                .filter(|&p| p > 0)
                .ok_or("The port is a number from 1 to 65535.")?,
        };
        let name = Some(value(&self.name))
            .filter(|n| !n.is_empty())
            .unwrap_or(host.clone());
        Ok(Site {
            id: self
                .editing
                .clone()
                .unwrap_or_else(|| store::site_id(&name, taken)),
            name,
            host,
            port,
            user: value(&self.user),
            remote_path: optional(&self.remote_path),
            local_path: optional(&self.local_path),
            key_path: optional(&self.key_path),
            use_agent: self.use_agent,
            color: self.color.map(|color| color.slug().to_string()),
        })
    }

    fn secrets(&self, cx: &gpui::App) -> (String, String) {
        (
            self.password.read(cx).value().to_string(),
            self.passphrase.read(cx).value().to_string(),
        )
    }
}

impl Workspace {
    pub fn open_form(
        &mut self,
        editing: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let site = editing
            .as_deref()
            .and_then(|id| self.site(id).cloned())
            .unwrap_or(Site {
                port: 22,
                use_agent: true,
                ..Default::default()
            });
        let (password, passphrase) = match &editing {
            Some(id) => (
                store::secret(id, Secret::Password)
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
                store::secret(id, Secret::Passphrase)
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
            ),
            None => Default::default(),
        };
        let mut field =
            |placeholder: &'static str, value: String, masked: bool| {
                let input = cx.new(|cx| {
                    InputState::new(window, cx)
                        .placeholder(placeholder)
                        .masked(masked)
                        .default_value(value)
                });
                cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
                    match event {
                        InputEvent::PressEnter { .. } => this.confirm_modal(window, cx),
                        InputEvent::Change => cx.notify(),
                        _ => {}
                    }
                })
                .detach();
                input
            };
        let form = SiteForm {
            editing: editing.clone(),
            name: field("My server", site.name.clone(), false),
            host: field("example.com", site.host.clone(), false),
            port: field("22", site.port.to_string(), false),
            user: field(
                &"user",
                if site.user.is_empty() && editing.is_none() {
                    std::env::var("USER").unwrap_or_default()
                } else {
                    site.user.clone()
                },
                false,
            ),
            password: field("Optional", password, true),
            key_path: field(
                "~/.ssh/id_ed25519 (default keys if empty)",
                site.key_path.clone().unwrap_or_default(),
                false,
            ),
            passphrase: field("Optional", passphrase, true),
            remote_path: field(
                "Login folder",
                site.remote_path.clone().unwrap_or_default(),
                false,
            ),
            local_path: field(
                "Unchanged",
                site.local_path.clone().unwrap_or_default(),
                false,
            ),
            use_agent: site.use_agent,
            color: site.color.as_deref().and_then(SiteColor::from_slug),
            error: None,
            testing: false,
            tested: None,
        };
        let focus = if editing.is_some() {
            form.name.clone()
        } else {
            form.host.clone()
        };
        focus.update(cx, |input, cx| input.focus(window, cx));
        self.modal = Some(Modal::Site(Box::new(form)));
        cx.notify();
    }

    /// Save the form; on a problem the form comes back with it written on.
    pub(super) fn submit_form(
        &mut self,
        mut form: Box<SiteForm>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), Box<SiteForm>> {
        let taken: Vec<String> = self.stored.sites.iter().map(|s| s.id.clone()).collect();
        let site = match form.site(cx, &taken) {
            Ok(site) => site,
            Err(error) => {
                form.error = Some(error);
                return Err(form);
            }
        };
        let (password, passphrase) = form.secrets(cx);
        let id = site.id.clone();
        let was_editing = form.editing.is_some();
        if let Err(error) = self.save_site(site, password, passphrase) {
            form.error = Some(error);
            return Err(form);
        }
        // An edit takes effect on the next connect, so a live session with the
        // old settings is closed rather than left looking current.
        if was_editing && self.conns.contains_key(&id) {
            self.disconnect(&id, cx);
        }
        if !was_editing {
            let active = self.active;
            self.open_site(id, active, cx);
        }
        Ok(())
    }

    fn test_form(&mut self, cx: &mut Context<Self>) {
        let Some(Modal::Site(form)) = &mut self.modal else {
            return;
        };
        let site = match form.site(cx, &[]) {
            Ok(site) => site,
            Err(error) => {
                form.tested = Some(Err(error));
                cx.notify();
                return;
            }
        };
        let (password, passphrase) = form.secrets(cx);
        form.testing = true;
        form.tested = None;
        let asker = self.asker.clone();
        let task = runtime().spawn(async move {
            let credentials = Credentials {
                password: Some(password),
                passphrase: Some(passphrase).filter(|p| !p.is_empty()),
            };
            let remote = Remote::connect(site, credentials, asker).await?;
            remote.disconnect().await;
            Ok::<(), String>(())
        });
        cx.spawn(async move |this, cx| {
            let result = task.await.unwrap_or_else(|error| Err(error.to_string()));
            this.update(cx, |this, cx| {
                if let Some(Modal::Site(form)) = &mut this.modal {
                    form.testing = false;
                    form.tested = Some(result);
                }
                cx.notify();
            })
        })
        .detach();
        cx.notify();
    }

    pub(super) fn render_form(
        &self,
        form: &SiteForm,
        t: Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let value = |input: &Entity<InputState>| input.read(cx).value().trim().to_string();
        let preview = Site {
            host: value(&form.host),
            port: value(&form.port).parse().unwrap_or(22),
            user: value(&form.user),
            ..Default::default()
        };
        let title = match value(&form.name) {
            name if !name.is_empty() => name,
            _ if form.editing.is_some() => "Edit site".into(),
            _ => "New site".into(),
        };
        let input = |state: &Entity<InputState>| {
            Input::new(state)
                .appearance(false)
                .p_0()
                .text_size(px(layout::TEXT_MD))
        };

        let header = div()
            .flex()
            .items_center()
            .gap(px(layout::SPACE_MD))
            .child(
                div()
                    .size(px(layout::FORM_LOGO_TILE))
                    .flex_shrink_0()
                    .rounded(px(layout::RADIUS_PANEL))
                    .bg(t.element_active)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        icon(icons::SERVER)
                            .size(px(layout::FORM_LOGO_SIZE))
                            .text_color(form.color.map(|c| c.swatch()).unwrap_or(t.text)),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(layout::FORM_TITLE_SIZE))
                            .line_height(px(layout::FORM_TITLE_LINE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(t.text)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(layout::TEXT_SM))
                            .font_family(crate::theme::MONO_FONT)
                            .text_color(t.text_muted)
                            .child(if preview.host.is_empty() {
                                "sftp://".to_string()
                            } else {
                                format!("sftp://{}", preview.address())
                            }),
                    ),
            )
            .child(
                ui::icon_button("form-close", icons::CLOSE, Tone::Quiet, Control::Compact, t)
                    .on_click(cx.listener(|this, _, window, cx| this.dismiss_modal(window, cx))),
            );

        let host_row = div()
            .h(px(layout::FORM_ROW_HEIGHT))
            .px(px(layout::SPACE_LG))
            .flex()
            .items_center()
            .gap(px(layout::SPACE_MD))
            .child(
                div()
                    .w(px(layout::FORM_LABEL_WIDTH))
                    .flex_shrink_0()
                    .text_size(px(layout::TEXT_MD))
                    .text_color(t.text_muted)
                    .child("Host"),
            )
            .child(div().flex_1().min_w_0().child(input(&form.host)))
            .child(
                div()
                    .w(px(1.))
                    .h(px(layout::CONTROL_HEIGHT_COMPACT))
                    .bg(t.border),
            )
            .child(
                div()
                    .w(px(layout::FORM_PORT_WIDTH))
                    .flex()
                    .items_center()
                    .gap(px(layout::SPACE_XS))
                    .child(div().text_color(t.text_faint).child(":"))
                    .child(input(&form.port)),
            )
            .into_any_element();

        let connection = ui::well(
            t,
            [
                ui::form_row(t, "Name", input(&form.name)),
                host_row,
                ui::form_row(t, "User", input(&form.user)),
                ui::form_row(t, "Password", input(&form.password)),
            ],
        );

        let agent = form.use_agent;
        let keys = ui::well(
            t,
            [
                ui::option_row(
                    t,
                    "SSH agent",
                    "Offer the identities the running ssh-agent holds.",
                    ui::switch("form-agent", agent, t).on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::Site(form)) = &mut this.modal {
                            form.use_agent = !form.use_agent;
                        }
                        cx.notify();
                    })),
                ),
                ui::form_row(t, "Key file", input(&form.key_path)),
                ui::form_row(t, "Passphrase", input(&form.passphrase)),
            ],
        );

        let swatches = div()
            .flex()
            .items_center()
            .gap(px(layout::SPACE_XS))
            .child(
                ui::swatch("color-none", None, form.color.is_none(), t).on_click(cx.listener(
                    |this, _, _, cx| {
                        if let Some(Modal::Site(form)) = &mut this.modal {
                            form.color = None;
                        }
                        cx.notify();
                    },
                )),
            )
            .children(SiteColor::ALL.into_iter().map(|color| {
                ui::swatch(
                    ("color", color as usize),
                    Some(color),
                    form.color == Some(color),
                    t,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(Modal::Site(form)) = &mut this.modal {
                        form.color = Some(color);
                    }
                    cx.notify();
                }))
            }));

        let places = ui::well(
            t,
            [
                ui::form_row(t, "Remote folder", input(&form.remote_path)),
                ui::form_row(t, "Local folder", input(&form.local_path)),
                div()
                    .h(px(layout::FORM_ROW_HEIGHT))
                    .px(px(layout::SPACE_LG))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(layout::TEXT_MD))
                            .text_color(t.text_muted)
                            .child("Color"),
                    )
                    .child(swatches)
                    .into_any_element(),
            ],
        );

        let test_note = match (&form.testing, &form.tested) {
            (true, _) => Some((t.text_muted, "Connecting…".to_string())),
            (false, Some(Ok(()))) => Some((t.success, "Connected and signed in.".to_string())),
            (false, Some(Err(error))) => Some((t.danger, error.clone())),
            _ => None,
        };

        let footer = div()
            .pt(px(layout::SPACE_MD))
            .border_t_1()
            .border_color(t.border)
            .flex()
            .items_center()
            .gap(px(layout::SPACE_SM))
            .child(
                ui::button("form-test", "Test", Tone::Primary, Control::Standard, t)
                    .disabled(form.testing)
                    .on_click(cx.listener(|this, _, _, cx| this.test_form(cx))),
            )
            .when_some(form.editing.clone(), |row, id| {
                row.child(
                    ui::button("form-delete", "Delete", Tone::Quiet, Control::Standard, t)
                        .on_click(cx.listener(move |this, _, _, cx| this.remove_site(&id, cx))),
                )
            })
            .child(div().flex_1())
            .child(
                ui::button("form-cancel", "Cancel", Tone::Quiet, Control::Standard, t)
                    .on_click(cx.listener(|this, _, window, cx| this.dismiss_modal(window, cx))),
            )
            .child(
                ui::accent_button(
                    "form-save",
                    if form.editing.is_some() {
                        "Save"
                    } else {
                        "Save & Connect"
                    },
                    t,
                    cx,
                )
                .on_click(cx.listener(|this, _, window, cx| this.confirm_modal(window, cx))),
            );

        div()
            .id("site-form")
            .occlude()
            .mt(px(layout::FORM_TOP))
            .w(px(layout::FORM_WIDTH))
            .max_h_full()
            .overflow_y_scroll()
            .p(px(layout::FORM_PADDING))
            .flex()
            .flex_col()
            .gap(px(layout::FORM_SECTION_GAP))
            .border_1()
            .border_color(t.border_strong)
            .rounded(px(layout::RADIUS_LARGE))
            .map(|card| ui::raised_card(card, t))
            .child(header)
            .child(section(t, "Connection", connection))
            .child(section(t, "Authentication", keys))
            .child(section(t, "Behaviour", places))
            .when_some(form.error.clone(), |card, error| {
                card.child(
                    div()
                        .text_size(px(layout::TEXT_SM))
                        .text_color(t.danger)
                        .child(error),
                )
            })
            .when_some(test_note, |card, (color, note)| {
                card.child(
                    div()
                        .text_size(px(layout::TEXT_SM))
                        .text_color(color)
                        .child(note),
                )
            })
            .child(footer)
            .into_any_element()
    }
}

fn section(t: Theme, label: &str, body: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(layout::SPACE_SM))
        .child(ui::section_label(t, label).pl(px(layout::SPACE_XS)))
        .child(body)
}
