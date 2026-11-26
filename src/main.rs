// SPDX-License-Identifier: GPL-3.0-only

mod app;
mod boards;

mod i18n;
mod widgets;

fn main() -> cosmic::iced::Result {
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    i18n::init(&requested_languages);

    let settings = cosmic::app::Settings::default()
        .size(cosmic::iced::Size::new(1240.0, 760.0))
        .size_limits(
            cosmic::iced::Limits::NONE
                .min_width(360.0)
                .min_height(180.0),
        );

    cosmic::app::run::<app::AppModel>(settings, ())
}
