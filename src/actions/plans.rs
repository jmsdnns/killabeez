use winnow::Result;
use winnow::prelude::*;
use winnow::{
    ascii::{line_ending, space0},
    combinator::{opt, repeat, terminated},
    token::take_while,
};

#[derive(Debug, PartialEq)]
pub enum ParsedAction {
    Execute { command: String },
    Upload { source: String },
    Download { source: String },
}

fn parse_execute(input: &mut &str) -> Result<ParsedAction> {
    "execute:".parse_next(input)?;
    space0.parse_next(input)?;
    let command = take_while(1.., |c| c != '\n').parse_next(input)?;

    Ok(ParsedAction::Execute {
        command: command.to_string(),
    })
}

fn parse_upload(input: &mut &str) -> Result<ParsedAction> {
    "upload:".parse_next(input)?;
    space0.parse_next(input)?;
    let source = take_while(1.., |c| c != '\n').parse_next(input)?;

    Ok(ParsedAction::Upload {
        source: source.to_string(),
    })
}

fn parse_download(input: &mut &str) -> Result<ParsedAction> {
    "download:".parse_next(input)?;
    space0.parse_next(input)?;
    let source = take_while(1.., |c| c != '\n').parse_next(input)?;

    Ok(ParsedAction::Download {
        source: source.to_string(),
    })
}

fn parse_command(input: &mut &str) -> Result<ParsedAction> {
    if input.len() >= 8 && &input.as_bytes()[0..8] == b"execute:" {
        parse_execute(input)
    } else if input.len() >= 7 && &input.as_bytes()[0..7] == b"upload:" {
        parse_upload(input)
    } else if input.len() >= 9 && &input.as_bytes()[0..9] == b"download:" {
        parse_download(input)
    } else {
        winnow::combinator::fail.parse_next(input)
    }
}

pub fn parse_commands(input: &mut &str) -> Result<Vec<ParsedAction>> {
    repeat(1.., terminated(parse_command, (space0, opt(line_ending)))).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_execute_command() {
        let mut input = "execute: echo \"take five\"\n";
        let result = parse_execute(&mut input).unwrap();
        assert_eq!(
            result,
            ParsedAction::Execute {
                command: "echo \"take five\"".to_string()
            }
        );
    }

    #[test]
    fn test_parse_single_upload_command() {
        let mut input = "upload: myscript.sh\n";
        let result = parse_upload(&mut input).unwrap();
        assert_eq!(
            result,
            ParsedAction::Upload {
                source: "myscript.sh".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_single_download_command() {
        let mut input = "download: /tmp/all-the-output.tar.gz\n";
        let result = parse_download(&mut input).unwrap();
        assert_eq!(
            result,
            ParsedAction::Download {
                source: "/tmp/all-the-output.tar.gz".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_example_plan() {
        let mut input = r#"execute: echo "Take Five!"
upload: my_init.sh
execute: chmod 755 my_init.sh && ./my_init.sh
execute: do_stuff
download: takefive.tar.gz
"#;
        let result = parse_commands(&mut input).unwrap();
        assert_eq!(
            result,
            vec![
                ParsedAction::Execute {
                    command: "echo \"Take Five!\"".to_string()
                },
                ParsedAction::Upload {
                    source: "my_init.sh".to_string(),
                },
                ParsedAction::Execute {
                    command: "chmod 755 my_init.sh && ./my_init.sh".to_string()
                },
                ParsedAction::Execute {
                    command: "do_stuff".to_string()
                },
                ParsedAction::Download {
                    source: "takefive.tar.gz".to_string()
                },
            ]
        );
    }
}
