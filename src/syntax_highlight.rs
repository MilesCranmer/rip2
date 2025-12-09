use nu_ansi_term::Color;
use syntect::highlighting::Style;

pub fn style_to_ansi(style: Style, text: &str) -> String {
    let mut ansi_style = nu_ansi_term::Style::new();

    /* Apply color */
    if let Some(fg_color) = to_ansi_color(style.foreground) {
        ansi_style = ansi_style.fg(fg_color);
    }

    if let Some(bg_color) = to_ansi_color(style.background) {
        ansi_style = ansi_style.on(bg_color);
    }

    /* Apply font styles */
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::BOLD)
    {
        ansi_style = ansi_style.bold();
    }
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::UNDERLINE)
    {
        ansi_style = ansi_style.underline();
    }
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::ITALIC)
    {
        ansi_style = ansi_style.italic();
    }

    ansi_style.paint(text).to_string()
}

// from: https://github.com/sharkdp/bat/blob/master/src/terminal.rs, ty for open source :)
fn to_ansi_color(color: syntect::highlighting::Color) -> Option<Color> {
    match color.a {
        // red channel encodes a palette index
        0 => Some(match color.r {
            0x00 => Color::Black,
            0x01 => Color::Red,
            0x02 => Color::Green,
            0x03 => Color::Yellow,
            0x04 => Color::Blue,
            0x05 => Color::Purple,
            0x06 => Color::Blue, // replace Cyan for better
            0x07 => Color::White,
            n => Color::Fixed(n),
        }),
        // use terminal default
        1 => None,
        // use rgb color
        _ => Some(Color::Rgb(color.r, color.g, color.b)),
    }
}
