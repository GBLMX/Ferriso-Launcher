//! Stable identity for projects that can come from more than one provider.

use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ProjectSource {
    #[default]
    Modrinth,
    CurseForge,
}

impl ProjectSource {
    pub const ALL: [Self; 2] = [Self::Modrinth, Self::CurseForge];

    pub fn label(self) -> &'static str {
        match self {
            Self::Modrinth => "Modrinth",
            Self::CurseForge => "CurseForge",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "modrinth" => Some(Self::Modrinth),
            "curseforge" => Some(Self::CurseForge),
            _ => None,
        }
    }
}

impl fmt::Display for ProjectSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_qualified_and_legacy_project_ids() {
        let curseforge = ProjectRef::parse("curseforge:123");
        assert_eq!(curseforge.source, ProjectSource::CurseForge);
        assert_eq!(curseforge.id, "123");

        let modrinth = ProjectRef::parse("AANobbMI");
        assert_eq!(modrinth.source, ProjectSource::Modrinth);
        assert_eq!(modrinth.id, "AANobbMI");
    }

    #[test]
    fn numeric_ids_remain_source_scoped() {
        let modrinth = ProjectRef::new(ProjectSource::Modrinth, "123");
        let curseforge = ProjectRef::new(ProjectSource::CurseForge, "123");
        assert_eq!(modrinth.id, curseforge.id);
        assert_ne!(modrinth.source, curseforge.source);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProjectRef {
    pub source: ProjectSource,
    pub id: String,
}

impl ProjectRef {
    pub fn new(source: ProjectSource, id: impl Into<String>) -> Self {
        Self {
            source,
            id: id.into(),
        }
    }

    /// Parses the CLI/deep-link form: plain IDs are Modrinth, while
    /// `curseforge:<id>` is explicit.
    pub fn parse(value: &str) -> Self {
        if let Some((prefix, id)) = value.split_once(':')
            && let Some(source) = ProjectSource::from_str(prefix)
        {
            return Self::new(source, id);
        }
        Self::new(ProjectSource::Modrinth, value)
    }
}
