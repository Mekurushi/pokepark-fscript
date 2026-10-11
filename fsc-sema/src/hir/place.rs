use super::LocalId;

//TODO: rethink the hardwiring to vec3 and hardcoded fields; generalize if other structured type
// comes up
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vec3Field {
    X,
    Y,
    Z,
}

impl Vec3Field {
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "x" => Some(Self::X),
            "y" => Some(Self::Y),
            "z" => Some(Self::Z),
            _ => None,
        }
    }

    #[must_use]
    pub const fn offset(self) -> u16 {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionElem {
    Vec3Field(Vec3Field),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub local: LocalId,
    pub projection: Vec<ProjectionElem>,
}

impl Place {
    #[must_use]
    pub const fn new(local: LocalId) -> Self {
        Self {
            local,
            projection: Vec::new(),
        }
    }

    #[must_use]
    pub fn project(mut self, projection: ProjectionElem) -> Self {
        self.projection.push(projection);
        self
    }
}
