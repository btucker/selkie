use crate::layout::LayoutDirection;

/// Shared diagram direction handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    TopToBottom,
    BottomToTop,
    LeftToRight,
    RightToLeft,
}

impl Direction {
    /// Parse direction from mermaid syntax (supports flowchart arrow hints).
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        if s.contains('<') {
            Self::RightToLeft
        } else if s.contains('^') {
            Self::BottomToTop
        } else if s.contains('>') {
            Self::LeftToRight
        } else if s.contains('v') || s == "TD" || s == "TB" {
            Self::TopToBottom
        } else {
            Self::from_str(s)
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "BT" => Self::BottomToTop,
            "LR" => Self::LeftToRight,
            "RL" => Self::RightToLeft,
            _ => Self::TopToBottom,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TopToBottom => "TB",
            Self::BottomToTop => "BT",
            Self::LeftToRight => "LR",
            Self::RightToLeft => "RL",
        }
    }
}

impl From<Direction> for LayoutDirection {
    fn from(direction: Direction) -> Self {
        match direction {
            Direction::TopToBottom => LayoutDirection::TopToBottom,
            Direction::BottomToTop => LayoutDirection::BottomToTop,
            Direction::LeftToRight => LayoutDirection::LeftToRight,
            Direction::RightToLeft => LayoutDirection::RightToLeft,
        }
    }
}
