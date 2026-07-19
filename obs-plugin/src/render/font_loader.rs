use std::io::Read;
use std::sync::mpsc::Receiver;
use std::time::Duration;

const MAX_BYTES: u64 = 20_000_000;
const FETCH_TIMEOUT: Duration = Duration::from_secs(8);

//overlay/scripts/consts.js FONT_FAMILY_LINKS
const GOOGLE_FONTS: &[(&str, &str)] = &[
    ("ArialPixel", "https://fonts.googleapis.com/css2?family=Geist+Pixel&display=swap"),
    ("Borevs", "https://fonts.googleapis.com/css2?family=IM+Fell+English+SC&display=swap"),
    ("abel", "https://fonts.googleapis.com/css2?family=Abel&display=swap"),
    ("archivo-black", "https://fonts.googleapis.com/css2?family=Archivo+Black&display=swap"),
    ("arimo", "https://fonts.googleapis.com/css2?family=Arimo:ital,wght@0,400..700;1,400..700&display=swap"),
    ("bebas-neue", "https://fonts.googleapis.com/css2?family=Bebas+Neue&display=swap"),
    ("bitcount-prop-single", "https://fonts.googleapis.com/css2?family=Bitcount+Prop+Single:wght@100..900&display=swap"),
    ("bungee", "https://fonts.googleapis.com/css2?family=Bungee&display=swap"),
    ("caveat-brush", "https://fonts.googleapis.com/css2?family=Caveat+Brush&display=swap"),
    ("chewy", "https://fonts.googleapis.com/css2?family=Chewy&display=swap"),
    ("cinzel", "https://fonts.googleapis.com/css2?family=Cinzel:wght@400..900&display=swap"),
    ("comfortaa", "https://fonts.googleapis.com/css2?family=Comfortaa:wght@300..700&display=swap"),
    ("fjalla-one", "https://fonts.googleapis.com/css2?family=Fjalla+One&display=swap"),
    ("fredoka", "https://fonts.googleapis.com/css2?family=Fredoka:wght@300..700&display=swap"),
    ("inter", "https://fonts.googleapis.com/css2?family=Inter:ital,opsz,wght@0,14..32,100..900;1,14..32,100..900&display=swap"),
    ("josefin-sans", "https://fonts.googleapis.com/css2?family=Josefin+Sans:ital,wght@0,100..700;1,100..700&display=swap"),
    ("knewave", "https://fonts.googleapis.com/css2?family=Knewave&display=swap"),
    (
        "lato",
        "https://fonts.googleapis.com/css2?family=Lato:ital,wght@0,100;0,300;0,400;0,700;0,900;1,100;1,300;1,400;1,700;1,900&display=swap",
    ),
    ("lexend", "https://fonts.googleapis.com/css2?family=Lexend:wght@100..900&display=swap"),
    ("lexend-giga", "https://fonts.googleapis.com/css2?family=Lexend+Giga:wght@100..900&display=swap"),
    ("lilita-one", "https://fonts.googleapis.com/css2?family=Lilita+One&display=swap"),
    ("montserrat", "https://fonts.googleapis.com/css2?family=Montserrat:ital,wght@0,100..900;1,100..900&display=swap"),
    ("noto-sans", "https://fonts.googleapis.com/css2?family=Noto+Sans:ital,wght@0,100..900;1,100..900&display=swap"),
    ("open-sans", "https://fonts.googleapis.com/css2?family=Open+Sans:ital,wght@0,300..800;1,300..800&display=swap"),
    ("orbitron", "https://fonts.googleapis.com/css2?family=Orbitron:wght@400..900&display=swap"),
    ("oswald", "https://fonts.googleapis.com/css2?family=Oswald:wght@200..700&display=swap"),
    ("pacifico", "https://fonts.googleapis.com/css2?family=Pacifico&display=swap"),
    ("press-start-2p", "https://fonts.googleapis.com/css2?family=Press+Start+2P&display=swap"),
    ("raleway", "https://fonts.googleapis.com/css2?family=Raleway:ital,wght@0,100..900;1,100..900&display=swap"),
    ("roboto-condensed", "https://fonts.googleapis.com/css2?family=Roboto+Condensed:ital,wght@0,100..900;1,100..900&display=swap"),
    ("roboto-mono", "https://fonts.googleapis.com/css2?family=Roboto+Mono:ital,wght@0,100..700;1,100..700&display=swap"),
    ("roboto", "https://fonts.googleapis.com/css2?family=Roboto:ital,wght@0,100..900;1,100..900&display=swap"),
    ("share-tech", "https://fonts.googleapis.com/css2?family=Share+Tech&display=swap"),
    ("smooch-sans", "https://fonts.googleapis.com/css2?family=Smooch+Sans:wght@100..900&display=swap"),
    ("sora", "https://fonts.googleapis.com/css2?family=Sora:wght@100..800&display=swap"),
    ("special-elite", "https://fonts.googleapis.com/css2?family=Special+Elite&display=swap"),
    ("titan-one", "https://fonts.googleapis.com/css2?family=Titan+One&display=swap"),
    (
        "ubuntu",
        "https://fonts.googleapis.com/css2?family=Ubuntu:ital,wght@0,300;0,400;0,500;0,700;1,300;1,400;1,500;1,700&display=swap",
    ),
];

fn google_font_url(fontfamily: &str) -> Option<&'static str> {
    GOOGLE_FONTS
        .iter()
        .find(|(k, _)| *k == fontfamily)
        .map(|(_, url)| *url)
}

pub fn spawn_load(fontfamily: &str, bold: bool) -> Option<Receiver<Option<Vec<u8>>>> {
    let url = google_font_url(fontfamily)?;
    let (tx, rx) = std::sync::mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("OverlayFont".into())
        .spawn(move || {
            let _ = tx.send(fetch(url, bold));
        })
        .is_ok();
    spawned.then_some(rx)
}

fn fetch(css_url: &str, bold: bool) -> Option<Vec<u8>> {
    let css = fetch_text(css_url)?;
    let font_url = pick_best_face(&css, bold)?;
    fetch_bytes(&font_url)
}

fn fetch_text(url: &str) -> Option<String> {
    ureq::get(url)
        .set(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 6.1; WOW64) AppleWebKit/537.36 (KHTML, like Gecko)",
        )
        .timeout(FETCH_TIMEOUT)
        .call()
        .ok()?
        .into_string()
        .ok()
}

fn fetch_bytes(url: &str) -> Option<Vec<u8>> {
    let response = ureq::get(url).timeout(FETCH_TIMEOUT).call().ok()?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(MAX_BYTES)
        .read_to_end(&mut bytes)
        .ok()?;
    Some(bytes)
}

fn pick_best_face(css: &str, bold: bool) -> Option<String> {
    let target: i32 = if bold { 700 } else { 400 };
    let mut best: Option<(bool, i32, String)> = None;

    for block in css.split("@font-face").skip(1) {
        let is_italic = block
            .find("font-style:")
            .map(|i| {
                block[i + "font-style:".len()..]
                    .trim_start()
                    .starts_with("italic")
            })
            .unwrap_or(false);

        let weight = block
            .find("font-weight:")
            .and_then(|i| {
                block[i + "font-weight:".len()..]
                    .trim_start()
                    .split(|c: char| !c.is_ascii_digit())
                    .next()
            })
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(400);

        let Some(url_start) = block.find("url(") else {
            continue;
        };
        let rest = &block[url_start + "url(".len()..];
        let Some(url_end) = rest.find(')') else {
            continue;
        };
        let url = rest[..url_end]
            .trim_matches('\'')
            .trim_matches('"')
            .to_string();

        let dist = (weight - target).abs();
        let better = match &best {
            Some((best_italic, best_dist, _)) => (is_italic, dist) < (*best_italic, *best_dist),
            None => true,
        };
        if better {
            best = Some((is_italic, dist, url));
        }
    }

    best.map(|(_, _, url)| url)
}
