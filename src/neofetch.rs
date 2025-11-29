use wasm_bindgen::prelude::*;

pub struct NeofetchLogo {
    pub lines: Vec<&'static str>,
    pub color: &'static str,
}

pub fn get_logo(os: &str) -> NeofetchLogo {
    if os.contains("Windows") {
        // Reverted to original detailed Windows ASCII logo; single tint color retained.
        NeofetchLogo {
            lines: vec![
                "                                ..,",
                "                    ....,,:;+ccllll",
                "      ...,,+:;  cllllllllllllllllll",
                ",cclllllllllll  lllllllllllllllllll",
                "llllllllllllll  lllllllllllllllllll",
                "llllllllllllll  lllllllllllllllllll",
                "llllllllllllll  lllllllllllllllllll",
                "llllllllllllll  lllllllllllllllllll",
                "llllllllllllll  lllllllllllllllllll",
                "                                   ",
                "llllllllllllll  lllllllllllllllllll",
                "llllllllllllll  lllllllllllllllllll",
                "llllllllllllll  lllllllllllllllllll",
                "llllllllllllll  lllllllllllllllllll",
                "llllllllllllll  lllllllllllllllllll",
                "`'ccllllllllll  lllllllllllllllllll",
                "       `' \\*::  :ccllllllllllllllll",
                "                       ````''*::cll",
                "                                 ``",
            ],
            color: "#00a4ef",
        }
    } else if os.contains("Mac") || os.contains("macOS") {
        NeofetchLogo {
            lines: vec![
                "                    'c.",
                "                 ,xNMM.",
                "               .OMMMMo",
                "               OMMM0,",
                "     .;loddo:' loolloddol;.",
                "   cKMMMMMMMMMMNWMMMMMMMMMM0:",
                " .KMMMMMMMMMMMMMMMMMMMMMMMWd.",
                " XMMMMMMMMMMMMMMMMMMMMMMMX.",
                ";MMMMMMMMMMMMMMMMMMMMMMMM:",
                ":MMMMMMMMMMMMMMMMMMMMMMMM:",
                ".MMMMMMMMMMMMMMMMMMMMMMMMX.",
                " kMMMMMMMMMMMMMMMMMMMMMMMMWd.",
                " .XMMMMMMMMMMMMMMMMMMMMMMMMMMk",
                "  .XMMMMMMMMMMMMMMMMMMMMMMMMK.",
                "    kMMMMMMMMMMMMMMMMMMMMMMd",
                "     ;KMMMMMMMWXXWMMMMMMMk.",
                "       .cooc,.    .,coo:.",
            ],
            color: "#ffffff",
        }
    } else if os.contains("Ubuntu") {
        NeofetchLogo {
            lines: vec![
                "            .-/+oossssoo+/-.",
                "        `:+ssssssssssssssssss+:`",
                "      -+ssssssssssssssssssyyssss+-",
                "    .osssssssssssssssssdMMMNysssso.",
                "   /ssssssssssshdmmNNmmyNMMMMhssssss/",
                "  +ssssssssshmydMMMMMMMNddddyssssssss+",
                " /sssssssshNMMMyhhyyyyhmNMMMNhssssssss/",
                ".ssssssssdMMMNhsssssssssshNMMMdssssssss.",
                "+sssshhhyNMMNyssssssssssssyNMMMysssssss+",
                "ossyNMMMNyMMhsssssssssssssshmmmhssssssso",
                "ossyNMMMNyMMhsssssssssssssshmmmhssssssso",
                "+sssshhhyNMMNyssssssssssssyNMMMysssssss+",
                ".ssssssssdMMMNhsssssssssshNMMMdssssssss.",
                " /sssssssshNMMMyhhyyyyhdNMMMNhssssssss/",
                "  +sssssssssdmydMMMMMMMMddddyssssssss+",
                "   /ssssssssssshdmNNNNmyNMMMMhssssss/",
                "    .osssssssssssssssssdMMMNysssso.",
                "      -+sssssssssssssssssyyyssss+-",
                "        `:+ssssssssssssssssss+:`",
                "            .-/+oossssoo+/-.",
            ],
            color: "#e95420",
        }
    } else if os.contains("Android") {
        NeofetchLogo {
            lines: vec![
                "         -o          o-",
                "          +hydNNNNdyh+",
                "        +mMMMMMMMMMMMMm+",
                "      `dMMm:NMMMMMMN:mMMd`",
                "      hMMMMMMMMMMMMMMMMMMh",
                "  ..  yyyyyyyyyyyyyyyyyyyy  ..",
                ".mMMm`MMMMMMMMMMMMMMMMMMMM`mMMm.",
                ":MMMM-MMMMMMMMMMMMMMMMMMMM-MMMM:",
                ":MMMM-MMMMMMMMMMMMMMMMMMMM-MMMM:",
                ".mMMm`MMMMMMMMMMMMMMMMMMMM`mMMm.",
                "  ..  yyyyyyyyyyyyyyyyyyyy  ..",
                "      hMMMMMMMMMMMMMMMMMMh",
                "      `dMMm:NMMMMMMN:mMMd`",
                "        +mMMMMMMMMMMMMm+",
                "          +hydNNNNdyh+",
                "         -o          o-",
            ],
            color: "#a4c639",
        }
    } else if os.contains("iOS") || os.contains("iPhone") || os.contains("iPad") {
        NeofetchLogo {
            lines: vec![
                "                    'c.",
                "                 ,xNMM.",
                "               .OMMMMo",
                "               lMMM\"",
                "     .;loddo:.  .olloddol;.",
                "   cKMMMMMMMMMMNWMMMMMMMMMM0:",
                " .KMMMMMMMMMMMMMMMMMMMMMMMWd.",
                " XMMMMMMMMMMMMMMMMMMMMMMMX.",
                ";MMMMMMMMMMMMMMMMMMMMMMMM:",
                ":MMMMMMMMMMMMMMMMMMMMMMMM:",
                ".MMMMMMMMMMMMMMMMMMMMMMMMX.",
                " kMMMMMMMMMMMMMMMMMMMMMMMMWd.",
                " 'XMMMMMMMMMMMMMMMMMMMMMMMMMMk",
                "  'XMMMMMMMMMMMMMMMMMMMMMMMMK.",
                "    kMMMMMMMMMMMMMMMMMMMMMMd",
                "     ;KMMMMMMMWXXWMMMMMMMk.",
                "       'cooc,.    .,coo:'",
            ],
            color: "#a2aaad",
        }
    } else {
        // Revert to original simple default Linux logo.
        NeofetchLogo {
            lines: vec![
                "        #####",
                "       #######",
                "       ##O#O##",
                "       #######",
                "     ###########",
                "    #############",
                "   ###############",
                "   ################",
                "  #################",
                "#####################",
                "#####################",
                "  #################",
            ],
            color: "#fcc421",
        }
    }
}

#[wasm_bindgen]
pub fn neofetch_logo(os: &str) -> String {
    let logo = get_logo(os);
    logo.lines.join("\n")
}

pub fn format_neofetch(
    os: &str,
    kernel: &str,
    browser: &str,
    cpu: &str,
    memory: &str,
    resolution: &str,
    uptime: &str,
) -> String {
    let logo = get_logo(os);
    let mut output = String::new();

    let info_lines = [
        "root@localhost".to_string(),
        "─────────────".to_string(),
        format!("OS: {}", os),
        format!("Host: {}", browser),
        format!("Kernel: {}", kernel),
        format!("Uptime: {}", uptime),
        "Shell: kpawnd-sh".to_string(),
        format!("Resolution: {}", resolution),
        format!("Terminal: {}", browser),
        format!("CPU: {}", cpu),
        format!("Memory: {}", memory),
    ];

    let max_logo_width = logo.lines.iter().map(|l| l.len()).max().unwrap_or(0);

    let empty_string = String::new();
    for i in 0..logo.lines.len().max(info_lines.len()) {
        let logo_line = logo.lines.get(i).unwrap_or(&"");
        let info_line = info_lines.get(i).unwrap_or(&empty_string);
        let padding = " ".repeat(max_logo_width - strip_color_tokens(logo_line).len() + 3);
        output.push_str(&format!(
            "\x1b[COLOR:{}]{}{}{}\n",
            logo.color, logo_line, padding, info_line
        ));
    }

    output
}

// Helper to measure visible length ignoring our color tokens.
fn strip_color_tokens(s: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if bytes[i] == 0x1b {
            // ESC
            // Look for pattern \x1b[COLOR:#......]
            if let Some(rest) = s.get(i..) {
                if rest.starts_with("\x1b[COLOR:#") {
                    // advance until closing ']'
                    if let Some(end) = rest.find(']') {
                        i += end + 1; // skip token
                        continue;
                    }
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}
