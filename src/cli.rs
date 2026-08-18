use std::path::PathBuf;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CliOptions {
    pub file: Option<PathBuf>,
    pub page: Option<i64>,
    pub tui_mode: bool,
}

impl CliOptions {
    pub fn parse_from_args<I, T>(args: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let mut opts = CliOptions::default();
        let args_vec: Vec<String> = args.into_iter().map(Into::into).collect();
        let mut iter = args_vec.into_iter().peekable();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--tui" => {
                    opts.tui_mode = true;
                }
                "--file" | "-f" => {
                    if let Some(val) = iter.next() {
                        opts.file = Some(PathBuf::from(val));
                    }
                }
                "--page" | "-p" => {
                    if let Some(val) = iter.next() {
                        if let Ok(p) = val.parse::<i64>() {
                            opts.page = Some(p);
                        }
                    }
                }
                other => {
                    if !other.starts_with('-') {
                        if opts.file.is_none() {
                            opts.file = Some(PathBuf::from(other));
                        } else if opts.page.is_none() {
                            if let Ok(p) = other.parse::<i64>() {
                                opts.page = Some(p);
                            }
                        }
                    }
                }
            }
        }
        opts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tui_and_flags() {
        let args = vec!["--file", "/tmp/comic.cbz", "--page", "15", "--tui"];
        let opts = CliOptions::parse_from_args(args);
        assert_eq!(
            opts,
            CliOptions {
                file: Some(PathBuf::from("/tmp/comic.cbz")),
                page: Some(15),
                tui_mode: true,
            }
        );
    }

    #[test]
    fn test_parse_positional_args() {
        let args = vec!["/tmp/chapter1.cbz", "5", "--tui"];
        let opts = CliOptions::parse_from_args(args);
        assert_eq!(
            opts,
            CliOptions {
                file: Some(PathBuf::from("/tmp/chapter1.cbz")),
                page: Some(5),
                tui_mode: true,
            }
        );
    }

    #[test]
    fn test_parse_file_only() {
        let args = vec!["/tmp/solo.cbz"];
        let opts = CliOptions::parse_from_args(args);
        assert_eq!(
            opts,
            CliOptions {
                file: Some(PathBuf::from("/tmp/solo.cbz")),
                page: None,
                tui_mode: false,
            }
        );
    }
}
