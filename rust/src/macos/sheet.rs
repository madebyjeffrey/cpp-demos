//! Native parent-modal ownership for portable alert transactions.
use super::*;

impl Surface {
    /// Present a native parent-modal sheet, retained until it completes or closes.
    /// The returned control handle need not be retained. Resize it if necessary.
    /// The alert's completion runs after the native sheet has released its parent.
    pub fn show_sheet(
        &self,
        alert: crate::ui::Alert,
        body: impl Fn() -> crate::ui::View + 'static,
    ) -> Result<Surface> {
        self.ensure_open()?;
        if alert.completed() {
            return Err(error("Alert is already completed"));
        }
        if self.state.native.attachedSheet().is_some() {
            return Err(error("A sheet is already presented for this parent"));
        }
        let app = self
            .state
            .app
            .upgrade()
            .ok_or_else(|| error("Application is closed"))?;
        let dialog = Dialog {
            surface: Surface {
                state: create_surface(
                    &app,
                    WindowOptions {
                        title: alert.title().into(),
                        size: Size::new(460., 300.),
                        minimum_size: Size::new(300., 160.),
                        ..Default::default()
                    },
                    Some(&self.state),
                )?,
            },
        };
        dialog.set_appearance(self.appearance());
        dialog.set_view({
            let alert = alert.clone();
            move || alert.sheet_view(body())
        })?;
        let weak = dialog.downgrade();
        let subscription = alert.observe_completion(move |_| {
            if let Some(surface) = weak.upgrade() {
                close_surface(&surface.state, DialogResult::Cancelled);
            }
        });
        dialog.state.sheet.replace(Some((alert, subscription)));
        dialog.show_modal(|_| {})?;
        let surface = dialog.surface_clone();
        if surface.is_open() {
            app.managed_dialogs.borrow_mut().push(dialog);
        }
        Ok(surface)
    }
}
