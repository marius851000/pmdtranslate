use std::str::FromStr;
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

#[derive(Debug, Error)]
/// A non fatal parsing error of Po file. The user should likely be informed of these
pub enum PoWarning {
    #[error("the line {0} start with the unknown symbol {1:?}")]
    LineTypeUnknown(usize, String),
    #[error("the line {0} end with an unclosed quote")]
    UnclosedQuote(usize),
    #[error("the line {0} end iwth an escape character")]
    UnfinishedEscape(usize),
}

#[derive(Default)]
pub struct GettextWriter {
    pub entrys: Vec<Entry>,
}

impl GettextWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn to_pot(&self) -> String {
        let mut result = String::new();

        for entry in &self.entrys {
            let text = format!("{} {} {}", entry.hash, entry.unk, entry.text);
            result.push_str(&format!(
                "#. {}\nmsgid {:?}\nmsgstr \"\"\n\n",
                &entry.source_file, text
            ))
        }

        result
    }

    pub fn from_po(file: String) -> (Self, Vec<PoWarning>) {
        let mut warning = Vec::new();
        let mut result = GettextWriter { entrys: Vec::new() };

        let mut msgid = String::new();
        let mut msgstr = String::new();
        let mut comment = String::new();
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
            |msgid: &mut String, msgstr: &mut String, comment: &mut String| {
                if msgid.is_empty() && msgstr.is_empty() && comment.is_empty() {
                    return;
                };
                if msgid.is_empty() {
                    todo!();
                };
                if msgstr.is_empty() {
                    let mut msgid_splited = msgid.split(' ');
                    let to_skip = msgid_splited.next().unwrap().len()
                        + 1
                        + msgid_splited.next().unwrap().len()
                        + 1;
                    *msgstr = msgid.chars().skip(to_skip).collect();
                };
                if comment.is_empty() {
                    todo!();
                };
                let mut msgid_splited_iterator = msgid.split(' ');
                if let Some(first_element) = msgid_splited_iterator.next() {
                    match u32::from_str(first_element) {
                        Ok(hash) => {
                            if let Some(second_element) = msgid_splited_iterator.next() {
                                match u32::from_str(second_element) {
                                    Ok(unk) => result.entrys.push(Entry {
                                        hash,
                                        unk,
                                        source_file: comment.to_string(),
                                        text: msgstr.to_string(),
                                    }),
                                    Err(_err) => todo!(),
                                }
                            }
                        }
                        Err(_err) => todo!(),
                    }
                } else {
                    todo!();
                }
                msgstr.clear();
                msgid.clear();
                comment.clear();
            };

        for (line_nb, line) in file.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            if let Some(first_command) = line.split(' ').next() {
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
                    comment = line.chars().skip(3).collect::<String>();
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
                let mut is_inside_quote = false;
                let mut next_escaped = false;
                for chara in line.chars() {
                    if next_escaped {
                        next_escaped = false;
                        match chara {
                            'n' => line_no_parentesis.push('\n'),
                            'r' => line_no_parentesis.push('\r'),
                            _ => line_no_parentesis.push(chara),
                        };
                    } else {
                        match chara {
                            '\\' => next_escaped = true,
                            '"' => is_inside_quote = !is_inside_quote,
                            _ => {
                                if is_inside_quote {
                                    line_no_parentesis.push(chara)
                                }
                            }
                        }
                    }
                }
                if is_inside_quote {
                    warning.push(PoWarning::UnclosedQuote(line_nb))
                }
                if next_escaped {
                    warning.push(PoWarning::UnfinishedEscape(line_nb))
                }
            };
        }
        push_current_translation(&mut msgid, &mut msgstr, &mut comment);

        (result, warning)
    }
}
