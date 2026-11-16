//! Endcord, Vencord and BetterDiscord themes: a `.theme.css` file, or a link
//! to one, read for its colours. Yapfast doesn't draw with CSS, so it reads the
//! custom properties a theme sets and maps them onto `ui/theme.slint`'s
//! colours: midnight's palette (`--bg-1`…`--bg-4`, `--text-0`…`--text-5`,
//! `--accent-1`…), which midnight and the themes built on it set, else
//! Discord's own variables. See docs/themes.md. Only std here, so the tests
//! run on their own.

use std::collections::HashMap;

/// A colour as red, green, blue and alpha, each 0 to 255.
pub type Rgba = [u8; 4];

/// The `http(s)` sheets `css` imports, in order.
pub fn imports(css: &str) -> Vec<String> {
    let css = without_comments(css);
    css.match_indices("@import")
        .filter_map(|(at, _)| {
            let rest = css[at + 7..].trim_start();
            let rest = rest.strip_prefix("url(").unwrap_or(rest).trim_start();
            let quote = rest.chars().next().filter(|c| matches!(c, '"' | '\''));
            let rest = quote.map_or(rest, |quote| &rest[quote.len_utf8()..]);
            let end = rest.find(|c: char| Some(c) == quote || matches!(c, ')' | ';' | ' ' | '\n'))?;
            let url = &rest[..end];
            (url.starts_with("https://") || url.starts_with("http://")).then(|| url.to_owned())
        })
        .collect()
}

/// A field of the theme's header comment, such as `@name`.
pub fn meta(css: &str, key: &str) -> Option<String> {
    let header = css.trim_start().strip_prefix("/*")?;
    let header = &header[..header.find("*/")?];
    let tag = format!("@{key}");
    header.lines().find_map(|line| {
        let line = line.trim_start().trim_start_matches('*').trim();
        let value = line.strip_prefix(&tag)?;
        (value.starts_with(char::is_whitespace) && !value.trim().is_empty())
            .then(|| value.trim().to_owned())
    })
}

fn without_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        rest = rest[start + 2..].find("*/").map_or("", |end| &rest[start + 2 + end + 2..]);
    }
    out.push_str(rest);
    out
}

/// The custom properties the sheets set, the later sheet's over the earlier
/// one's, leaving out those inside a light theme's rules.
#[derive(Default)]
pub struct Vars(HashMap<String, String>);

impl Vars {
    /// Takes the custom properties `css` sets, over those taken before.
    pub fn read(&mut self, css: &str) {
        let css = without_comments(css);
        let bytes = css.as_bytes();
        // Whether each open block is a light theme's.
        let mut light = vec![false];
        let mut start = 0;
        let mut at = 0;
        while at < bytes.len() {
            match bytes[at] {
                b'{' => {
                    let selector = css[start..at].to_lowercase();
                    let parent = *light.last().unwrap_or(&false);
                    light.push(parent || selector.contains("theme-light") || selector.contains(".light"));
                    start = at + 1;
                }
                b'}' => {
                    if light.len() > 1 {
                        light.pop();
                    }
                    start = at + 1;
                }
                b';' => start = at + 1,
                b'-' if bytes.get(at + 1) == Some(&b'-')
                    && css[start..at].trim().is_empty() =>
                {
                    let name_end = css[at + 2..]
                        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
                        .map_or(css.len(), |end| at + 2 + end);
                    let name = &css[at + 2..name_end];
                    let after = css[name_end..].trim_start();
                    if let Some(value) = after.strip_prefix(':') {
                        let value_start = css.len() - value.len();
                        let value_end = value_end(&css, value_start);
                        if !*light.last().unwrap_or(&false) && !name.is_empty() {
                            let value = css[value_start..value_end].trim();
                            let value = value.strip_suffix("!important").unwrap_or(value).trim();
                            self.0.insert(name.to_owned(), value.to_owned());
                        }
                        at = value_end;
                        start = at;
                        continue;
                    }
                }
                _ => {}
            }
            at += 1;
        }
    }

    /// The colour a property comes to, following `var()`s; `None` for one
    /// not set, or not a colour Yapfast reads, such as a gradient.
    pub fn colour(&self, name: &str) -> Option<Rgba> {
        colour(&self.resolve(self.0.get(name)?, 0)?)
    }

    /// The first of `names` that comes to a colour.
    pub fn first(&self, names: &[&str]) -> Option<Rgba> {
        names.iter().find_map(|name| self.colour(name))
    }

    fn resolve(&self, value: &str, depth: u32) -> Option<String> {
        if depth > 12 {
            return None;
        }
        let Some(at) = value.find("var(") else {
            return Some(value.to_owned());
        };
        let inner_start = at + 4;
        let inner_end = inner_start + closing(&value[inner_start..])?;
        let inner = &value[inner_start..inner_end];
        let (name, fallback) = match top_level_comma(inner) {
            Some(comma) => (inner[..comma].trim(), Some(inner[comma + 1..].trim())),
            None => (inner.trim(), None),
        };
        let replaced = match self.0.get(name.trim_start_matches("--")) {
            Some(found) => self.resolve(found, depth + 1)?,
            None => self.resolve(fallback?, depth + 1)?,
        };
        let joined = format!("{}{replaced}{}", &value[..at], &value[inner_end + 1..]);
        self.resolve(&joined, depth + 1)
    }
}

/// Where a declaration's value starting at `from` ends: the `;` or `}` outside
/// brackets and quotes.
fn value_end(css: &str, from: usize) -> usize {
    let mut depth = 0i32;
    let mut quote = None;
    for (offset, c) in css[from..].char_indices() {
        match (quote, c) {
            (Some(open), c) if c == open => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '(') => depth += 1,
            (None, ')') => depth -= 1,
            (None, ';' | '}') if depth <= 0 => return from + offset,
            _ => {}
        }
    }
    css.len()
}

/// Where the bracket opened just before `text` closes, in `text`.
fn closing(text: &str) -> Option<usize> {
    let mut depth = 0;
    for (at, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(at),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn top_level_comma(text: &str) -> Option<usize> {
    let mut depth = 0;
    for (at, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => return Some(at),
            _ => {}
        }
    }
    None
}

/// A CSS colour: `#hex`, `rgb()`, `hsl()`, `oklch()`, `color-mix()` in sRGB,
/// or `black`, `white` or `transparent`.
pub fn colour(text: &str) -> Option<Rgba> {
    let text = text.trim().to_lowercase();
    if let Some(hex) = text.strip_prefix('#') {
        let digits: Vec<u8> = hex
            .chars()
            .map(|c| c.to_digit(16).map(|d| d as u8))
            .collect::<Option<_>>()?;
        let pair = |i: usize| digits[i] * 16 + digits[i + 1];
        return match digits.len() {
            3 | 4 => {
                let mut out = [255; 4];
                for (slot, digit) in out.iter_mut().zip(&digits) {
                    *slot = digit * 17;
                }
                Some(out)
            }
            6 => Some([pair(0), pair(2), pair(4), 255]),
            8 => Some([pair(0), pair(2), pair(4), pair(6)]),
            _ => None,
        };
    }
    match text.as_str() {
        "black" => return Some([0, 0, 0, 255]),
        "white" => return Some([255; 4]),
        "transparent" => return Some([0; 4]),
        _ => {}
    }
    let open = text.find('(')?;
    let function = &text[..open];
    let inner = &text[open + 1..open + 1 + closing(&text[open + 1..])?];
    if function == "color-mix" {
        return mix(inner);
    }
    // `a, b, c, d` or `a b c / d`.
    let parts: Vec<&str> = inner
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 3 {
        return None;
    }
    let number = |part: &str, whole: f32| -> Option<f32> {
        match part.strip_suffix('%') {
            Some(percent) => percent.parse::<f32>().ok().map(|p| p / 100.0 * whole),
            None => part.trim_end_matches("deg").parse().ok(),
        }
    };
    let alpha = match parts.get(3) {
        Some(part) => number(part, 1.0)?.clamp(0.0, 1.0),
        None => 1.0,
    };
    let [r, g, b] = match function {
        "rgb" | "rgba" => [
            number(parts[0], 255.0)? / 255.0,
            number(parts[1], 255.0)? / 255.0,
            number(parts[2], 255.0)? / 255.0,
        ],
        "hsl" | "hsla" => hsl(
            number(parts[0], 360.0)?,
            number(parts[1], 100.0)? / 100.0,
            number(parts[2], 100.0)? / 100.0,
        ),
        "oklch" => oklch(
            number(parts[0], 1.0)?,
            number(parts[1], 0.4)?,
            number(parts[2], 360.0)?,
        ),
        _ => return None,
    };
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    Some([byte(r), byte(g), byte(b), byte(alpha)])
}

fn hsl(hue: f32, saturation: f32, lightness: f32) -> [f32; 3] {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hue = hue.rem_euclid(360.0) / 60.0;
    let x = chroma * (1.0 - (hue % 2.0 - 1.0).abs());
    let (r, g, b) = match hue as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = lightness - chroma / 2.0;
    [r + m, g + m, b + m]
}

/// OKLCH to sRGB, as CSS Color 4 gives the matrices.
fn oklch(lightness: f32, chroma: f32, hue: f32) -> [f32; 3] {
    let (a, b) = (
        chroma * hue.to_radians().cos(),
        chroma * hue.to_radians().sin(),
    );
    let l = (lightness + 0.396_337_78 * a + 0.215_803_76 * b).powi(3);
    let m = (lightness - 0.105_561_346 * a - 0.063_854_17 * b).powi(3);
    let s = (lightness - 0.089_484_18 * a - 1.291_485_5 * b).powi(3);
    let linear = [
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s,
        -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    ];
    linear.map(|value| {
        if value <= 0.003_130_8 {
            12.92 * value
        } else {
            1.055 * value.powf(1.0 / 2.4) - 0.055
        }
    })
}

/// `color-mix(in SPACE, A [p%], B [q%])`, mixed in sRGB whatever the space.
fn mix(inner: &str) -> Option<Rgba> {
    let first = top_level_comma(inner)?;
    let rest = &inner[first + 1..];
    let second = top_level_comma(rest)?;
    let side = |text: &str| -> Option<(Rgba, Option<f32>)> {
        let text = text.trim();
        match text.rsplit_once(' ') {
            Some((colour_text, percent)) if percent.ends_with('%') => Some((
                colour(colour_text)?,
                percent.trim_end_matches('%').parse::<f32>().ok().map(|p| p / 100.0),
            )),
            _ => Some((colour(text)?, None)),
        }
    };
    let (a, pa) = side(&rest[..second])?;
    let (b, pb) = side(&rest[second + 1..])?;
    let wa = match (pa, pb) {
        (Some(pa), _) => pa,
        (None, Some(pb)) => 1.0 - pb,
        (None, None) => 0.5,
    };
    Some(blend(a, b, wa))
}

/// `a` weighted `wa` and `b` the rest, premultiplied, so mixing with
/// `transparent` fades.
fn blend(a: Rgba, b: Rgba, wa: f32) -> Rgba {
    let wb = 1.0 - wa;
    let alpha = a[3] as f32 * wa + b[3] as f32 * wb;
    let channel = |i: usize| {
        if alpha <= 0.0 {
            return 0;
        }
        let sum = a[i] as f32 * a[3] as f32 * wa + b[i] as f32 * b[3] as f32 * wb;
        (sum / alpha).round().clamp(0.0, 255.0) as u8
    };
    [channel(0), channel(1), channel(2), alpha.round() as u8]
}

/// `top` laid over the opaque `under`.
pub fn over(top: Rgba, under: Rgba) -> Rgba {
    let a = top[3] as f32 / 255.0;
    let channel = |i: usize| (top[i] as f32 * a + under[i] as f32 * (1.0 - a)).round() as u8;
    [channel(0), channel(1), channel(2), 255]
}

/// `colour` at `alpha` (0 to 255).
fn faded(colour: Rgba, alpha: u8) -> Rgba {
    [colour[0], colour[1], colour[2], alpha]
}

/// Whether `colour` is dark enough for light text.
pub fn dark(colour: Rgba) -> bool {
    let luma = 0.2126 * colour[0] as f32 + 0.7152 * colour[1] as f32 + 0.0722 * colour[2] as f32;
    luma < 128.0
}

/// The colours a theme's properties give, by their names in
/// `ui/theme.slint`; a colour it doesn't give keeps the base theme's.
pub fn colours(vars: &Vars) -> Vec<(&'static str, Rgba)> {
    let mut out = Vec::new();
    if vars.colour("bg-4").is_some() && vars.colour("text-3").is_some() {
        midnight(vars, &mut out);
    } else {
        discord(vars, &mut out);
    }
    // Grounds see-through themes leave clear, as glass, go solid.
    for (name, colour) in &mut out {
        let clear = matches!(*name, "accent-soft" | "mentioned" | "mentioned-hover" | "shimmer");
        if !clear && colour[3] < 255 {
            *colour = over(*colour, [0, 0, 0, 255]);
        }
    }
    out
}

fn midnight(vars: &Vars, out: &mut Vec<(&'static str, Rgba)>) {
    let bg = |n: u8| vars.colour(&format!("bg-{n}"));
    let text = |n: u8| vars.colour(&format!("text-{n}"));
    let accent = |n: u8| vars.colour(&format!("accent-{n}"));
    let Some(ground) = bg(4) else { return };
    let chat = bg(3).unwrap_or(ground);
    let field = bg(2).unwrap_or(chat);
    let mut put = |name: &'static str, colour: Option<Rgba>| {
        if let Some(colour) = colour {
            out.push((name, colour));
        }
    };
    put("rail", Some(ground));
    put("sidebar", Some(ground));
    put("sunken", Some(ground));
    put("edge", Some(ground));
    put("panel", Some(chat));
    put("chat", Some(chat));
    put("stage", Some(chat));
    put("tooltip", Some(chat));
    put("raised", Some(field));
    put("field", Some(field));
    put("button-default", Some(field));
    put("button-default-hover", bg(1).or(Some(field)));
    put("hover", vars.colour("hover").map(|hover| over(hover, ground)));
    put("active", vars.colour("active").map(|active| over(active, ground)));
    put("row-hover", vars.colour("message-hover").map(|hover| over(hover, chat)));
    let border = vars.first(&["border", "active"]);
    put("line", border.map(|line| over(line, chat)));
    put("outline", vars.first(&["border-light", "hover"]).map(|line| over(line, ground)));
    put("outline-chat", border.map(|line| over(line, chat)));
    put("outline-field", border.map(|line| over(line, field)));
    put("text", text(2).or(text(1)));
    put("secondary", text(3));
    put("muted", text(4));
    put("faint", text(5));
    put("reply-spine", text(5));
    put("accent", accent(3).or(accent(2)));
    put("accent-hover", accent(4).or(accent(3)));
    put("on-accent", text(0));
    put("link", accent(1));
    if let Some(soft) = accent(2) {
        put("accent-soft", Some(faded(soft, 0x26)));
        put("mentioned", Some(faded(soft, 0x1f)));
        put("mentioned-hover", Some(faded(soft, 0x29)));
    }
    put("danger", vars.first(&["red-3", "dnd", "accent-new"]));
    put("danger-hover", vars.first(&["red-4", "red-3", "dnd"]));
    put("on-danger", text(0));
    put("live", vars.first(&["green-3", "online"]));
    put("loud", vars.first(&["yellow-3", "idle"]));
    let light = text(1).unwrap_or([255; 4]);
    put("overlay", Some(faded(light, 255)));
    put("shimmer", Some(faded(light, 0x10)));
}

fn discord(vars: &Vars, out: &mut Vec<(&'static str, Rgba)>) {
    let ground = vars.first(&["background-base-lowest", "background-tertiary"]);
    let sidebar = vars.first(&["background-base-lowest", "background-secondary"]).or(ground);
    let chat = vars.first(&["chat-background-default", "background-base-lower", "background-primary"]);
    let low = vars.first(&["background-base-low", "background-secondary-alt"]);
    let field = vars
        .first(&["channeltextarea-background", "input-background", "background-base-low"])
        .or(low);
    let mut put = |name: &'static str, colour: Option<Rgba>| {
        if let Some(colour) = colour {
            out.push((name, colour));
        }
    };
    put("rail", ground);
    put("sidebar", sidebar);
    put("sunken", ground);
    put("chat", chat);
    put("stage", chat);
    put("panel", low);
    put("field", field);
    put("raised", vars.first(&["background-surface-high", "background-floating"]));
    put("tooltip", vars.first(&["background-floating", "background-surface-highest"]));
    let on = |names: &[&str], under: Option<Rgba>| Some(over(vars.first(names)?, under?));
    put("hover", on(&["background-modifier-hover", "interactive-background-hover"], sidebar));
    put("active", on(&["background-modifier-selected", "interactive-background-selected"], sidebar));
    put("row-hover", on(&["background-message-hover", "background-modifier-hover"], chat));
    let line = on(&["border-subtle", "background-modifier-accent"], chat);
    put("line", line);
    put("outline-chat", line);
    put("outline", on(&["border-subtle", "background-modifier-accent"], sidebar));
    put("outline-field", on(&["border-subtle", "background-modifier-accent"], field));
    put("text", vars.first(&["text-default", "text-normal", "header-primary"]));
    put("secondary", vars.first(&["text-secondary", "header-secondary", "interactive-normal"]));
    put("muted", vars.first(&["text-muted", "channels-default"]));
    put("faint", vars.first(&["interactive-muted", "channels-muted", "text-muted"]));
    put("link", vars.first(&["text-link"]));
    let accent = vars.first(&["control-primary-background-default", "brand-500", "brand-experiment"]);
    put("accent", accent);
    put(
        "accent-hover",
        vars.first(&["control-primary-background-hover", "brand-560", "brand-experiment-560"]),
    );
    if let Some(accent) = accent {
        put("accent-soft", Some(faded(accent, 0x26)));
    }
    put("danger", vars.first(&["control-critical-primary-background-default", "status-danger"]));
    put("live", vars.first(&["status-positive", "status-green-600"]));
    put("loud", vars.first(&["status-warning"]));
}

#[cfg(test)]
mod tests {
    use super::*;

    const INKSHELL: &str = r#"/**
 * @name Inkshell
 * @author someone
*/
@import url('https://markchan0225.github.io/RoundedDiscord/src/roundeddiscord.css');
@import url("https://refact0r.github.io/midnight-discord/build/midnight.css");
body {
    --dms-icon-svg-url: url('data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg"><path d="M1 2Z"/></svg>');
    --text-0: var(--bg-4); /* text on colored elements */
    --text-1: #F4F4F5;
    --text-2: #F4F4F5;
    --text-3: #B9B9BE;
    --text-4: #85858B;
    --text-5: #5A5A5F;
    --bg-1: #3A3A3E;
    --bg-2: #242427;
    --bg-3: #161618;
    --bg-4: #000000;
    --hover: rgba(244, 244, 245, 0.06);
    --active: rgba(244, 244, 245, 0.11);
    --accent-2: #F4F4F5;
    --accent-3: #F4F4F5;
    --red-3: oklch(65% 0.12 0);
}
.theme-light { --bg-4: #ffffff; }
"#;

    fn hex(colour: Rgba) -> String {
        format!("#{:02x}{:02x}{:02x}{:02x}", colour[0], colour[1], colour[2], colour[3])
    }

    #[test]
    fn reads_a_midnight_theme() {
        assert_eq!(meta(INKSHELL, "name").as_deref(), Some("Inkshell"));
        assert_eq!(
            imports(INKSHELL),
            [
                "https://markchan0225.github.io/RoundedDiscord/src/roundeddiscord.css",
                "https://refact0r.github.io/midnight-discord/build/midnight.css"
            ]
        );
        let mut vars = Vars::default();
        vars.read(INKSHELL);
        assert_eq!(vars.colour("bg-4"), Some([0, 0, 0, 255]), "the light rule is left out");
        assert_eq!(vars.colour("text-0"), Some([0, 0, 0, 255]), "through var()");
        let colours: HashMap<_, _> = colours(&vars).into_iter().collect();
        assert_eq!(hex(colours["rail"]), "#000000ff");
        assert_eq!(hex(colours["chat"]), "#161618ff");
        assert_eq!(hex(colours["text"]), "#f4f4f5ff");
        assert_eq!(hex(colours["secondary"]), "#b9b9beff");
        // 6% of #f4f4f5 over black.
        assert_eq!(hex(colours["hover"]), "#0e0e0eff");
        assert!(dark(colours["chat"]));
        let red = colours["danger"];
        assert!(red[0] > 180 && red[1] < 140, "oklch red: {}", hex(red));
    }

    #[test]
    fn reads_colours() {
        assert_eq!(colour("#abc"), Some([0xaa, 0xbb, 0xcc, 255]));
        assert_eq!(colour("#11223344"), Some([0x11, 0x22, 0x33, 0x44]));
        assert_eq!(colour("rgb(255 0 0 / 50%)"), Some([255, 0, 0, 128]));
        assert_eq!(colour("hsl(0, 100%, 50%)"), Some([255, 0, 0, 255]));
        assert_eq!(colour("hsla(220, 15%, 10%, 1)"), Some([22, 24, 29, 255]));
        assert_eq!(colour("oklch(100% 0 0)"), Some([255, 255, 255, 255]));
        assert_eq!(
            colour("color-mix(in hsl, #ff0000, transparent 90%)"),
            Some([255, 0, 0, 26])
        );
        assert_eq!(colour("linear-gradient(red, blue)"), None);
    }

    #[test]
    fn reads_discord_variables() {
        let mut vars = Vars::default();
        vars.read(
            ":root { --background-primary: #36393f; --background-secondary: #2f3136; \
             --background-tertiary: #202225; --text-normal: #dcddde; --brand-experiment: #5865f2 }",
        );
        let colours: HashMap<_, _> = colours(&vars).into_iter().collect();
        assert_eq!(hex(colours["chat"]), "#36393fff");
        assert_eq!(hex(colours["rail"]), "#202225ff");
        assert_eq!(hex(colours["accent"]), "#5865f2ff");
    }
}
