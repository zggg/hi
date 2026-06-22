use super::id::MessageId;
use super::locale::Locale;

pub fn format(locale: Locale, id: MessageId, args: &[String]) -> String {
    match locale {
        Locale::Zh => format_zh(id, args),
        Locale::En => format_en(id, args),
    }
}

mod zh;
mod en;

use zh::format_zh;
use en::format_en;
