use std::{borrow::Cow, collections::BTreeSet, str::FromStr};
use thiserror::Error;

pub struct Entry {
    pub text: String,
    pub hash: u32,
    pub unk: u32,
    pub source_file: String,
}

impl Entry {
    pub fn new(text: String, hash: u32, unk: u32, source_file: String) -> Self {
        Self {
            text,
            hash,
            unk,
            source_file,
        }
    }
}

const DISCRIMINATOR: &str = "ŧdiscrimatorŧ";

pub struct EntryNoText<'a> {
    pub hash: u32,
    pub unk: u32,
    pub source_file: &'a str,
}

impl<'a> EntryNoText<'a> {
    /// Take an [`Entry`], convert it to an [`EntryNoText`], also returning the text as the second output.
    pub fn from_entry(entry: &'a Entry) -> (Self, &'a str) {
        (
            Self {
                hash: entry.hash,
                unk: entry.unk,
                source_file: &entry.source_file,
            },
            &entry.text,
        )
    }
}

#[derive(Debug, Error)]
/// A non fatal parsing error of Po file. The user should likely be informed of these
pub enum PoWarning {
    #[error("the line {0} start with the unknown symbol {1:?}")]
    LineTypeUnknown(usize, String),
    #[error("the line {0} end with an unclosed quote")]
    UnclosedQuote(usize),
    #[error("the line {0} end with an escape character")]
    UnfinishedEscape(usize),
}

pub struct GettextWriter {
    pub entries: Vec<Entry>,
    discriminated: BTreeSet<String>,
}

pub fn escape_string_for_gettext(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + 10);
    result.push('"');
    for ch in text.chars() {
        if ch == '"' {
            result.push_str("\\\"");
        } else if ch == '\n' {
            result.push_str("\\n");
        } else if ch == '\r' {
            result.push_str("\\\\r");
        } else if ch == '\\' {
            result.push_str("\\\\");
        } else if ch.is_control() {
            result.push_str(&format!("\\\\x{}{:x}{}", '{', ch as u32, '}'));
        } else {
            result.push(ch);
        }
    }
    result.push('"');
    result
}

impl GettextWriter {
    pub fn new(discriminated: Vec<String>) -> Self {
        Self {
            entries: Vec::new(),
            discriminated: discriminated
                .into_iter()
                .map(|x| x.to_lowercase())
                .collect(),
        }
    }

    pub fn to_pot(&self) -> String {
        let mut result = String::new();

        // deduplicate the strings

        let mut translate_string: Vec<(Cow<str>, Vec<EntryNoText>)> = Vec::new();

        for entry in &self.entries {
            let (entry_no_text, text) = EntryNoText::from_entry(entry);
            let mut should_be_discriminated = false;
            let text_lower = text.to_lowercase();
            for discriminator in &self.discriminated {
                if text_lower.contains(discriminator) {
                    should_be_discriminated = true;
                }
            }
            let text = if should_be_discriminated {
                Cow::from(format!(
                    "{}{} {} {}",
                    text, DISCRIMINATOR, entry_no_text.source_file, entry_no_text.hash
                ))
            } else {
                Cow::from(text)
            };

            let mut insert_at = None;
            for entry in translate_string.iter_mut() {
                if entry.0 == text {
                    insert_at = Some(entry);
                    break;
                }
            }
            if let Some(good) = insert_at {
                good.1.push(entry_no_text)
            } else {
                translate_string.push((text, vec![entry_no_text]))
            }
        }

        // create the po file

        for entry in translate_string {
            for source in entry.1 {
                result.push_str(&format!(
                    "#. {} {} {}\n",
                    source.source_file, source.hash, source.unk
                ));
            }
            result.push_str(&format!(
                "msgid {}\nmsgstr \"\"\n\n",
                escape_string_for_gettext(if entry.0 == "" { " " } else { entry.0.as_ref() })
            ));
        }

        result
    }

    pub fn from_po(file: String) -> (Self, Vec<PoWarning>) {
        let mut warning = Vec::new();
        let mut result = GettextWriter {
            entries: Vec::new(),
            discriminated: BTreeSet::default(),
        };

        let mut msgid = String::new();
        let mut msgstr = String::new();
        let mut comment = Vec::new();
        pub enum Parsing {
            Msgid,
            Msgstr,
        }
        let mut parsing: Option<Parsing> = None;

        #[derive(Clone, PartialEq)]
        pub enum Phase {
            Pre,
            Final,
        }
        let mut phase = Phase::Pre; //passing from data to comment mean we are into the next part

        let mut push_current_translation =
            |msgid_input: &mut String, msgstr: &mut String, comment: &mut Vec<String>| {
                if msgid_input.is_empty() && comment.is_empty() {
                    return;
                };
                if msgid_input == " " {
                    msgid_input.clear();
                    msgstr.clear();
                };
                let msgid = msgid_input.split(DISCRIMINATOR).next().unwrap().to_string();
                if msgstr.is_empty() {
                    *msgstr = msgid.clone();
                };
                if comment.is_empty() {
                    todo!("the comment for \"{}\" is empty", msgid_input);
                };
                for comment_line in comment.iter() {
                    let mut line_splited = comment_line.split(' ');
                    let file_source = line_splited.next().unwrap();
                    let hash = u32::from_str(line_splited.next().unwrap()).unwrap();
                    let unk1 = u32::from_str(line_splited.next().unwrap()).unwrap();
                    result.entries.push(Entry {
                        text: msgstr.clone(),
                        hash,
                        unk: unk1,
                        source_file: file_source.to_string(),
                    });
                }

                msgstr.clear();
                msgid_input.clear();
                comment.clear();
            };

        for (line_nb, line) in file.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            if Some('\"') == line.chars().next() {
            } else if let Some(first_command) = line.split(' ').next() {
                let next_phase = match first_command {
                    "msgstr" => {
                        parsing = Some(Parsing::Msgstr);
                        Phase::Final
                    }
                    "msgid" => {
                        parsing = Some(Parsing::Msgid);
                        Phase::Pre
                    }
                    "#." => Phase::Pre,
                    _ => {
                        warning.push(PoWarning::LineTypeUnknown(
                            line_nb,
                            first_command.to_string(),
                        ));
                        phase.clone()
                    }
                };
                if phase == Phase::Final && next_phase == Phase::Pre {
                    push_current_translation(&mut msgid, &mut msgstr, &mut comment);
                };
                if first_command == "#." {
                    comment.push(line.chars().skip(3).collect::<String>());
                };
                phase = next_phase;
            } else {
                continue;
            };

            if let Some(what_is_parsed) = &parsing {
                let line_no_parentesis = match what_is_parsed {
                    Parsing::Msgid => &mut msgid,
                    Parsing::Msgstr => &mut msgstr,
                };

                let mut inside_quote = false;
                #[derive(PartialEq)]
                enum Escape {
                    None,
                    NextEscaped,
                    SecondEscaped,
                    WillBeEscaped,
                    ReadingNumber,
                }
                let mut escape = Escape::None;
                let mut number_being_read = String::new();

                for ch in line.chars() {
                    if escape == Escape::SecondEscaped {
                        if ch == 'x' {
                            escape = Escape::WillBeEscaped;
                            continue;
                        } else if ch == 'r' {
                            escape = Escape::None;
                            line_no_parentesis.push('\r');
                        } else {
                            escape = Escape::None;
                            line_no_parentesis.push('\\');
                        };
                    }

                    if escape == Escape::NextEscaped {
                        escape = Escape::None;
                        match ch {
                            'n' => line_no_parentesis.push('\n'),
                            'r' => line_no_parentesis.push('\r'),
                            '\\' => escape = Escape::SecondEscaped,
                            ch => line_no_parentesis.push(ch),
                        }
                    } else if escape == Escape::WillBeEscaped {
                        if ch == '{' {
                            escape = Escape::ReadingNumber;
                        } else {
                            todo!("error message");
                        }
                    } else if escape == Escape::ReadingNumber {
                        if ch == '}' {
                            let new_ch_text = u32::from_str_radix(&number_being_read, 16).unwrap();
                            let new_ch = char::from_u32(new_ch_text).unwrap();
                            line_no_parentesis.push(new_ch);
                            escape = Escape::None;
                            number_being_read = String::new();
                        } else {
                            number_being_read.push(ch);
                        }
                    } else if ch == '"' {
                        inside_quote = !inside_quote;
                    } else if inside_quote {
                        if ch == '\\' {
                            escape = Escape::NextEscaped;
                        } else if inside_quote {
                            line_no_parentesis.push(ch);
                        };
                    }
                }

                if escape == Escape::SecondEscaped {
                    line_no_parentesis.push('\\');
                }
            };
        }
        push_current_translation(&mut msgid, &mut msgstr, &mut comment);

        (result, warning)
    }

    pub fn merge(&mut self, other: Self) {
        for entry in other.entries {
            self.entries.push(entry);
        }
    }
}
