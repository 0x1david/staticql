#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use logging::bail_with;
use regex::Regex;

use crate::finder_types::FinderType;

pub fn format_python_string(
    format_str: &str,
    args: &[FinderType],
    kwargs: &[(String, FinderType)],
) -> Option<String> {
    let re = Regex::new(
        r"%\(([^)]+)\)[-+0 #]*(?:\*|\d+)?(?:\.(?:\*|\d+))?[hlL]?[sdifgGeEoxXcubp%]|%[-+0 #]*(?:\*|\d+)?(?:\.(?:\*|\d+))?[hlL]?[sdifgGeEoxXcubp%]",
    ).ok()?;
    let mut result = format_str.to_string();
    let mut value_index = 0;
    let matches: Vec<_> = re.find_iter(format_str).collect();

    for m in matches.iter().rev() {
        let specifier = m.as_str();

        if specifier == "%%" {
            result.replace_range(m.range(), "%");
            continue;
        }

        let (value, conv) = if specifier.starts_with("%(") {
            // Named format specifier like %(name)s
            let key_end = specifier.find(')')?;
            let key = &specifier[2..key_end]; // Extract key between %( and )
            let conv = specifier.chars().last()?;

            // Find the value in kwargs slice
            let value = kwargs.iter().find(|(k, _)| k == key).map(|(_, v)| v)?;
            (value, conv)
        } else {
            // Positional format specifier like %s, %d, etc.
            if value_index >= args.len() {
                return None;
            }
            let value = &args[args.len() - 1 - value_index];
            value_index += 1;
            let conv = specifier.chars().last()?;
            (value, conv)
        };

        let replacement = match conv {
            's' => Some(value.to_string()),
            'd' | 'i' => format_value_as_int(value),
            'u' => format_value_as_unsigned(value),
            'b' => format_value_as_binary(value),
            'f' | 'F' => format_value_as_float(value, specifier),
            'g' | 'G' => format_value_as_general(value, specifier),
            'e' | 'E' => format_value_as_scientific(value, specifier),
            'o' => format_value_as_octal(value),
            'x' => format_value_as_hex(value, false),
            'X' => format_value_as_hex(value, true),
            'c' => format_value_as_char(value),
            'p' => format_value_as_pointer(value),
            _ => bail_with!(None, "Unhandled format conversion specifier: {}", conv),
        };

        result.replace_range(m.range(), &replacement?);
    }

    Some(result)
}

fn format_value_as_unsigned(value: &FinderType) -> Option<String> {
    match value {
        FinderType::Int(i) => i.parse::<u64>().ok().map(|i| i.to_string()),
        FinderType::Float(f) => Some((*f as u64).to_string()),
        FinderType::Bool(b) => Some(if *b { "1".to_string() } else { "0".to_string() }),
        FinderType::Str(s) => s.parse::<u64>().ok().map(|i| i.to_string()),
        _ => bail_with!(None, "Unhandled unsigned value formatting: {value}"),
    }
}

fn format_value_as_binary(value: &FinderType) -> Option<String> {
    match value {
        FinderType::Int(i) => i.parse::<i64>().ok().map(|i| format!("{i:b}")),
        FinderType::Float(f) => Some(format!("{:b}", *f as i64)),
        FinderType::Bool(b) => Some(if *b { "1".to_string() } else { "0".to_string() }),
        _ => bail_with!(None, "Unhandled binary value formatting: {value}"),
    }
}

fn format_value_as_general(value: &FinderType, specifier: &str) -> Option<String> {
    let precision = extract_precision(specifier).unwrap_or(6);
    let uppercase = specifier.contains('G');

    match value {
        FinderType::Float(f) => Some(format_general_float(*f, precision, uppercase)),
        FinderType::Int(i) => i
            .parse::<f64>()
            .ok()
            .map(|f| format_general_float(f, precision, uppercase)),
        FinderType::Bool(b) => {
            let val = if *b { 1.0 } else { 0.0 };
            Some(format_general_float(val, precision, uppercase))
        }
        FinderType::Str(s) => s
            .parse::<f64>()
            .ok()
            .map(|f| format_general_float(f, precision, uppercase)),
        _ => bail_with!(None, "Unhandled general value formatting: {value}"),
    }
}
fn format_general_float(f: f64, precision: usize, uppercase: bool) -> String {
    let abs_f = f.abs();
    let exponent = if abs_f == 0.0 {
        0
    } else {
        abs_f.log10().floor() as i32
    };

    if exponent < -4 || exponent >= precision as i32 {
        if uppercase {
            format!("{:.prec$E}", f, prec = precision.saturating_sub(1))
        } else {
            format!("{:.prec$e}", f, prec = precision.saturating_sub(1))
        }
    } else {
        let formatted = format!(
            "{:.prec$}",
            f,
            prec = precision
                .saturating_sub(1)
                .saturating_sub(exponent.max(0) as usize)
        );

        if formatted.contains('.') {
            formatted
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        } else {
            formatted
        }
    }
}

fn format_value_as_float(value: &FinderType, specifier: &str) -> Option<String> {
    let precision = extract_precision(specifier).unwrap_or(6);
    match value {
        FinderType::Float(f) => Some(format!("{f:.precision$}")),
        FinderType::Int(i) => i.parse::<f64>().ok().map(|f| format!("{f:.precision$}")),
        FinderType::Bool(b) => Some(if *b {
            format!("{:.precision$}", 1.0)
        } else {
            format!("{:.precision$}", 0.0)
        }),
        FinderType::Str(s) => s.parse::<f64>().ok().map(|f| format!("{f:.precision$}")),
        _ => bail_with!(None, "Unhandled float value formatting: {value}"),
    }
}

fn format_value_as_pointer(value: &FinderType) -> Option<String> {
    match value {
        FinderType::Int(i) => i.parse::<usize>().ok().map(|i| format!("0x{i:x}")),
        FinderType::Float(f) => Some(format!("0x{:x}", *f as usize)),
        _ => bail_with!(None, "Unhandled pointer value formatting: {value}"),
    }
}

fn extract_precision(specifier: &str) -> Option<usize> {
    specifier.find('.').and_then(|dot_pos| {
        let after_dot = &specifier[dot_pos + 1..];
        after_dot
            .find(|c: char| c.is_alphabetic())
            .and_then(|end| after_dot[..end].parse().ok())
    })
}
fn format_value_as_int(value: &FinderType) -> Option<String> {
    match value {
        FinderType::Int(i) => Some(i.clone()),
        FinderType::Float(f) => Some((*f as i64).to_string()),
        FinderType::Bool(b) => Some(if *b { "1".to_string() } else { "0".to_string() }),
        FinderType::Str(s) => s.parse::<i64>().ok().map(|i| i.to_string()),
        _ => bail_with!(None, "Unhandled integer value formatting: {value}"),
    }
}

fn format_value_as_octal(value: &FinderType) -> Option<String> {
    match value {
        FinderType::Int(i) => i.parse::<i64>().ok().map(|i| format!("{i:o}")),
        FinderType::Float(f) => Some(format!("{:o}", *f as i64)),
        FinderType::Bool(b) => Some(if *b { "1".to_string() } else { "0".to_string() }),
        _ => bail_with!(None, "Unhandled octal value formatting: {value}"),
    }
}

fn format_value_as_hex(value: &FinderType, uppercase: bool) -> Option<String> {
    match value {
        FinderType::Int(i) => i.parse::<i64>().ok().map(|i| {
            if uppercase {
                format!("{i:X}")
            } else {
                format!("{i:x}")
            }
        }),
        FinderType::Float(f) => Some(if uppercase {
            format!("{:X}", *f as i64)
        } else {
            format!("{:x}", *f as i64)
        }),
        FinderType::Bool(b) => Some(if *b { "1".to_string() } else { "0".to_string() }),
        _ => bail_with!(None, "Unhandled hex value formatting: {value}"),
    }
}
fn format_value_as_scientific(value: &FinderType, specifier: &str) -> Option<String> {
    let precision = extract_precision(specifier).unwrap_or(6);
    let uppercase = specifier.contains('E');

    match value {
        FinderType::Float(f) => {
            if uppercase {
                Some(format!("{f:.precision$E}"))
            } else {
                Some(format!("{f:.precision$e}"))
            }
        }
        FinderType::Int(i) => i.parse::<f64>().ok().map(|f| {
            if uppercase {
                format!("{f:.precision$E}")
            } else {
                format!("{f:.precision$e}")
            }
        }),
        FinderType::Bool(b) => {
            let val = if *b { 1.0 } else { 0.0 };
            if uppercase {
                Some(format!("{val:.precision$E}"))
            } else {
                Some(format!("{val:.precision$e}"))
            }
        }
        FinderType::Str(s) => s.parse::<f64>().ok().map(|f| {
            if uppercase {
                format!("{f:.precision$E}")
            } else {
                format!("{f:.precision$e}")
            }
        }),
        _ => bail_with!(None, "Unhandled scientific value formatting: {value}"),
    }
}

fn format_value_as_char(value: &FinderType) -> Option<String> {
    match value {
        FinderType::Int(i) => {
            if let Ok(code) = i.parse::<u32>() {
                if let Some(ch) = char::from_u32(code) {
                    return Some(ch.to_string());
                }
            }
            None
        }
        FinderType::Str(s) => {
            if s.len() == 1 {
                Some(s.clone())
            } else {
                None
            }
        }
        _ => bail_with!(None, "Unhandled char value formatting: {value}"),
    }
}
