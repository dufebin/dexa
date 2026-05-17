use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Mode {
    #[default]
    Human,
    Fast,
}

impl FromStr for Mode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "human" => Ok(Mode::Human),
            "fast" => Ok(Mode::Fast),
            other => Err(anyhow::anyhow!(
                "invalid mode '{}', expected 'human' or 'fast'",
                other
            )),
        }
    }
}
