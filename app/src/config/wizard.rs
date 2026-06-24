use std::fmt::Display;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

use hi_core::{t, Config, Locale, MessageId};

/// Author: gz
pub struct SelectOption<'a> {
    pub value: &'a str,
    pub label: &'a str,
    pub hint: &'a str,
}

/// Interactive config wizard session (`hi setup` / `hi gateway setup`).
///
/// Author: gz
pub struct Session {
    pub config_path: PathBuf,
    locale: Locale,
    interactive: bool,
}

impl Session {
    pub fn new(config_path: PathBuf, locale: Locale) -> Self {
        Self {
            config_path,
            locale,
            interactive: io::stdin().is_terminal() && io::stdout().is_terminal(),
        }
    }

    pub fn locale(&self) -> Locale {
        self.locale
    }

    pub fn start(&self, title: &str) -> anyhow::Result<()> {
        if self.interactive {
            cliclack::intro(title)?;
        } else {
            println!();
            println!("{title}");
            println!(
                "{}",
                t(
                    self.locale(),
                    MessageId::WizardConfigPathLine,
                    &[self.config_path.display().to_string()],
                )
            );
            println!();
        }
        Ok(())
    }

    pub fn note(&self, title: &str, body: &str) -> anyhow::Result<()> {
        if self.interactive {
            cliclack::note(body, title)?;
        } else {
            println!("[{title}]");
            for line in body.lines() {
                println!("  {line}");
            }
            println!();
        }
        Ok(())
    }

    pub fn select<'a>(
        &self,
        message: &str,
        options: &[SelectOption<'a>],
        default: &'a str,
    ) -> anyhow::Result<&'a str> {
        self.select_with(message, options, default, true)
    }

    /// Like [`select`](Self::select), but can force the menu even when only one option exists.
    pub fn select_with<'a>(
        &self,
        message: &str,
        options: &[SelectOption<'a>],
        default: &'a str,
        skip_if_single: bool,
    ) -> anyhow::Result<&'a str> {
        if skip_if_single && options.len() == 1 {
            return Ok(options[0].value);
        }

        if options.is_empty() {
            anyhow::bail!(
                "{}",
                t(
                    self.locale(),
                    MessageId::WizardSelectEmpty,
                    &[message.to_string()],
                )
            );
        }

        if self.interactive {
            let mut prompt = cliclack::select(message).initial_value(default);
            for opt in options {
                prompt = prompt.item(opt.value, opt.label, opt.hint);
            }
            return self.map_cancel(prompt.interact());
        }

        println!("{message}");
        for (idx, opt) in options.iter().enumerate() {
            println!("  {}) {} — {}", idx + 1, opt.label, opt.hint);
        }
        let default_idx = options
            .iter()
            .position(|o| o.value == default)
            .map(|i| (i + 1).to_string())
            .unwrap_or_else(|| "1".into());
        let prompt = t(
            self.locale(),
            MessageId::WizardSelectNumberPrompt,
            std::slice::from_ref(&default_idx),
        );
        let raw = plain_line(&prompt, &default_idx)?;
        let n: usize = raw
            .parse::<usize>()
            .unwrap_or(1)
            .saturating_sub(1)
            .min(options.len().saturating_sub(1));
        Ok(options[n].value)
    }

    /// Text input; empty line submits `default`.
    pub fn input(&self, message: &str, default: &str) -> anyhow::Result<String> {
        if self.interactive {
            return self.map_cancel(
                cliclack::input(message)
                    .default_input(default)
                    .interact(),
            );
        }
        Ok(plain_line(message, default)?)
    }

    /// Optional text; empty line keeps `default`, or `None` when `default` is empty.
    pub fn input_optional(&self, message: &str, default: &str) -> anyhow::Result<Option<String>> {
        let value = self.input(message, default)?;
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    pub fn password(&self, message: &str) -> anyhow::Result<String> {
        if self.interactive {
            return self.map_cancel(cliclack::password(message).interact());
        }
        print!("{message}: ");
        io::stdout().flush()?;
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        Ok(buf.trim().to_string())
    }

    /// Masked secret; empty line keeps the existing value when already configured.
    pub fn password_keep(&self, message: &str, configured: bool, current: &str) -> anyhow::Result<String> {
        if !configured {
            return self.password(message);
        }
        let label = format!(
            "{message}{}",
            t(self.locale(), MessageId::WizardPasswordKeepSuffix, &[])
        );
        if self.interactive {
            let value = self.map_cancel(cliclack::password(&label).allow_empty().interact())?;
            if value.is_empty() {
                Ok(current.to_string())
            } else {
                Ok(value)
            }
        } else {
            print!("{label}: ");
            io::stdout().flush()?;
            let mut buf = String::new();
            io::stdin().read_line(&mut buf)?;
            let buf = buf.trim();
            if buf.is_empty() {
                Ok(current.to_string())
            } else {
                Ok(buf.to_string())
            }
        }
    }

    pub fn confirm(&self, message: &str, default: bool) -> anyhow::Result<bool> {
        if self.interactive {
            return self.map_cancel(cliclack::confirm(message).initial_value(default).interact());
        }
        let hint = if default { "Y/n" } else { "y/N" };
        let raw = plain_line(&format!("{message} [{hint}]"), hint)?;
        Ok(match raw.to_lowercase().as_str() {
            "y" | "yes" => true,
            "n" | "no" => false,
            "" => default,
            _ => default,
        })
    }

    pub fn save<F>(&self, label: &str, write: F) -> anyhow::Result<()>
    where
        F: FnOnce() -> hi_core::Result<()>,
    {
        if self.interactive {
            let spin = cliclack::spinner();
            spin.start(label);
            let result = write();
            match result {
                Ok(()) => {
                    spin.stop(t(
                        self.locale,
                        MessageId::WizardWritten,
                        &[self.config_path.display().to_string()],
                    ));
                }
                Err(e) => {
                    spin.stop(t(self.locale, MessageId::WizardWriteFailed, &[]));
                    return Err(anyhow::anyhow!(e.to_string()));
                }
            }
            return Ok(());
        }
        write().map_err(|e| anyhow::anyhow!(e.to_string()))?;
        println!(
            "{}",
            t(
                self.locale,
                MessageId::WizardWritten,
                &[self.config_path.display().to_string()],
            )
        );
        Ok(())
    }

    /// 以加载动画包裹一段阻塞工作（如网络拉取模型列表）；
    /// 闭包返回 `(结果, 结束提示)`，结束提示在动画停止时显示。
    pub fn spinner<T>(&self, start_label: &str, work: impl FnOnce() -> (T, String)) -> T {
        if self.interactive {
            let spin = cliclack::spinner();
            spin.start(start_label);
            let (result, done_label) = work();
            spin.stop(done_label);
            result
        } else {
            println!("{start_label}");
            let (result, done_label) = work();
            println!("{done_label}");
            result
        }
    }

    pub fn finish(&self, message: &str) -> anyhow::Result<()> {
        if self.interactive {
            cliclack::outro_note(
                t(self.locale(), MessageId::WizardFinishTitle, &[]),
                message,
            )?;
        } else {
            println!();
            println!("{message}");
            println!();
        }
        Ok(())
    }

    fn map_cancel<T, E: Display>(&self, result: Result<T, E>) -> anyhow::Result<T> {
        result.map_err(|e| {
            if self.interactive {
                let _ = cliclack::outro_cancel(t(
                    self.locale(),
                    MessageId::WizardCancelled,
                    &[],
                ));
            }
            anyhow::anyhow!("{e}")
        })
    }
}

/// Author: gz
pub fn summarize_config(locale: Locale, config: &Config) -> String {
    let key = if config.ai.api_key.trim().is_empty() {
        t(locale, MessageId::SetupSummaryApiKeyMissing, &[])
    } else {
        t(locale, MessageId::SetupSummaryMaskedKey, &[])
    };
    format!(
        "{}\n{}\n{}\n{}",
        t(
            locale,
            MessageId::SetupSummaryProvider,
            std::slice::from_ref(&config.ai.provider),
        ),
        t(
            locale,
            MessageId::SetupSummaryModel,
            std::slice::from_ref(&config.ai.model),
        ),
        t(
            locale,
            MessageId::SetupSummaryWorkspace,
            std::slice::from_ref(&config.workspace),
        ),
        key
    )
}

/// Setup summary including configured message channels.
///
/// Author: gz
pub fn summarize_setup(locale: Locale, config: &Config, channels_summary: &str) -> String {
    format!(
        "{}\n{}",
        summarize_config(locale, config),
        t(
            locale,
            MessageId::SetupSummaryChannels,
            &[channels_summary.to_string()],
        )
    )
}

fn plain_line(label: &str, default: &str) -> io::Result<String> {
    print!("{label}: ");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    let buf = buf.trim();
    if buf.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(buf.to_string())
    }
}

#[cfg(test)]
#[path = "../../test/unit/config/wizard.rs"]
mod tests;
