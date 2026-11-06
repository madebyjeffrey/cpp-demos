use super::*;
pub fn view(m: &Lab) -> View {
    let readonly = m.readonly.get();
    let color = m.color.clone().read_only(readonly);
    let (well, well_popup) = color.well(&m.color_popup, "lab-color");
    grid(vec![
        card(
            m,
            "date",
            "Localized date · de-DE",
            m.date.clone().read_only(readonly).view(),
        ),
        card(
            m,
            "time",
            "Time · 12-hour",
            m.time.clone().read_only(readonly).view(),
        ),
        card(
            m,
            "datetime",
            "Local date and time",
            m.datetime.clone().read_only(readonly).view(),
        ),
        card(
            m,
            "zoned",
            "Zoned date and time",
            m.zoned.clone().read_only(readonly).view(),
        ),
        card(
            m,
            "color",
            "RGB / HSV / hex and alpha",
            View::layer(vec![
                View::column(vec![well, color.view()]).gap(12.),
                well_popup,
            ]),
        ),
    ])
}
