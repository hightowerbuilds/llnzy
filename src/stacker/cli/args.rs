use std::path::PathBuf;

use super::super::storage::PromptState;

#[derive(Debug)]
pub(super) struct AddArgs {
    pub(super) label: String,
    pub(super) category: Option<String>,
    pub(super) workspace: Option<String>,
    pub(super) source_agent: Option<String>,
    pub(super) session_id: Option<String>,
    pub(super) file: Option<PathBuf>,
    pub(super) body: Option<String>,
}

#[derive(Debug)]
pub(super) struct ListArgs {
    pub(super) state: PromptCliState,
    pub(super) format: ListFormat,
}

#[derive(Debug)]
pub(super) struct EditArgs {
    pub(super) id: String,
    pub(super) state: PromptCliState,
    pub(super) label: Option<String>,
    pub(super) category: Option<String>,
    pub(super) file: Option<PathBuf>,
    pub(super) body: Option<String>,
    pub(super) read_stdin: bool,
}

#[derive(Debug)]
pub(super) struct DeleteArgs {
    pub(super) id: String,
    pub(super) state: PromptCliState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PromptCliState {
    Saved,
    Inbox,
    Archive,
}

impl PromptCliState {
    fn from_flag(value: &str) -> Result<Self, String> {
        match value {
            "saved" => Ok(Self::Saved),
            "inbox" | "pending" => Ok(Self::Inbox),
            "archive" | "archived" => Ok(Self::Archive),
            other => Err(format!("unknown state: {other}")),
        }
    }

    pub(super) fn storage_state(self) -> PromptState {
        match self {
            Self::Saved => PromptState::Saved,
            Self::Inbox => PromptState::Pending,
            Self::Archive => PromptState::Archived,
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Saved => "saved",
            Self::Inbox => "inbox",
            Self::Archive => "archive",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ListFormat {
    Text,
    Json,
}

impl ListFormat {
    fn from_flag(value: &str) -> Result<Self, String> {
        match value {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(format!("unknown format: {other}")),
        }
    }
}

pub(super) fn parse_add_flags(args: &[String]) -> Result<AddArgs, String> {
    let mut label: Option<String> = None;
    let mut category = None;
    let mut workspace = None;
    let mut source_agent = None;
    let mut session_id = None;
    let mut file = None;
    let mut body = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--label" => label = Some(next_value(&mut iter, "--label")?),
            "--category" => category = Some(next_value(&mut iter, "--category")?),
            "--workspace" => workspace = Some(next_value(&mut iter, "--workspace")?),
            "--source-agent" => source_agent = Some(next_value(&mut iter, "--source-agent")?),
            "--session" => session_id = Some(next_value(&mut iter, "--session")?),
            "--file" => file = Some(PathBuf::from(next_value(&mut iter, "--file")?)),
            "--body" => body = Some(next_value(&mut iter, "--body")?),
            other => return Err(format!("unknown flag: {other}")),
        }
    }

    if file.is_some() && body.is_some() {
        return Err("choose only one of --body or --file".to_string());
    }

    let label = label.ok_or_else(|| "missing required --label".to_string())?;
    Ok(AddArgs {
        label,
        category,
        workspace,
        source_agent,
        session_id,
        file,
        body,
    })
}

pub(super) fn parse_list_flags(args: &[String]) -> Result<ListArgs, String> {
    let mut state = PromptCliState::Saved;
    let mut format = ListFormat::Text;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--state" => state = PromptCliState::from_flag(&next_value(&mut iter, "--state")?)?,
            "--format" => format = ListFormat::from_flag(&next_value(&mut iter, "--format")?)?,
            other => return Err(format!("unknown flag: {other}")),
        }
    }

    Ok(ListArgs { state, format })
}

pub(super) fn parse_edit_flags(args: &[String]) -> Result<EditArgs, String> {
    let mut id = None;
    let mut state = PromptCliState::Saved;
    let mut label = None;
    let mut category = None;
    let mut file = None;
    let mut body = None;
    let mut read_stdin = false;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--state" => state = PromptCliState::from_flag(&next_value(&mut iter, "--state")?)?,
            "--label" => label = Some(next_value(&mut iter, "--label")?),
            "--category" => category = Some(next_value(&mut iter, "--category")?),
            "--file" => file = Some(PathBuf::from(next_value(&mut iter, "--file")?)),
            "--body" => body = Some(next_value(&mut iter, "--body")?),
            "--stdin" => read_stdin = true,
            other if other.starts_with("--") => return Err(format!("unknown flag: {other}")),
            other => {
                if id.is_some() {
                    return Err(format!("unexpected argument: {other}"));
                }
                id = Some(other.to_string());
            }
        }
    }

    let id = parse_prompt_id(&id.ok_or_else(|| "missing prompt id".to_string())?)?;
    Ok(EditArgs {
        id,
        state,
        label,
        category,
        file,
        body,
        read_stdin,
    })
}

pub(super) fn parse_delete_flags(args: &[String]) -> Result<DeleteArgs, String> {
    let mut id = None;
    let mut state = PromptCliState::Saved;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--state" => state = PromptCliState::from_flag(&next_value(&mut iter, "--state")?)?,
            other if other.starts_with("--") => return Err(format!("unknown flag: {other}")),
            other => {
                if id.is_some() {
                    return Err(format!("unexpected argument: {other}"));
                }
                id = Some(other.to_string());
            }
        }
    }

    let id = parse_prompt_id(&id.ok_or_else(|| "missing prompt id".to_string())?)?;
    Ok(DeleteArgs { id, state })
}

fn parse_prompt_id(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.parse::<ulid::Ulid>().is_err() {
        return Err("invalid prompt id".to_string());
    }
    Ok(trimmed.to_string())
}

fn next_value(iter: &mut std::slice::Iter<'_, String>, flag: &str) -> Result<String, String> {
    iter.next()
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}
