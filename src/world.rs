use bfge::math::Vec3;

/// Неподвижная игровая область в мировых координатах; декор сюда не входит.
#[derive(Clone, Copy)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn center(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(self) -> Vec3 {
        self.max - self.min
    }
}

#[derive(Clone, Copy)]
pub struct StaticBox {
    pub center: Vec3,
    pub size: Vec3,
}

impl StaticBox {
    pub fn bounds(self) -> Aabb {
        Aabb {
            min: self.center - self.size * 0.5,
            max: self.center + self.size * 0.5,
        }
    }
}

// Плоскость земли рисуется на верхней грани этого объёма.
pub const GROUND: StaticBox = StaticBox {
    center: Vec3::new(40.0, -0.1, 0.0),
    size: Vec3::new(160.0, 0.2, 200.0),
};

pub const PLATFORMS: [StaticBox; 5] = [
    StaticBox {
        center: Vec3::new(-0.5, 0.25, -1.0),
        size: Vec3::new(1.4, 0.5, 1.4),
    },
    StaticBox {
        center: Vec3::new(1.5, 0.5, 2.5),
        size: Vec3::new(1.4, 1.0, 1.4),
    },
    StaticBox {
        center: Vec3::new(4.0, 0.75, -1.5),
        size: Vec3::new(1.4, 1.5, 1.4),
    },
    StaticBox {
        center: Vec3::new(6.0, 1.0, 2.5),
        size: Vec3::new(1.4, 2.0, 1.4),
    },
    StaticBox {
        center: Vec3::new(8.0, 1.25, -1.5),
        size: Vec3::new(1.4, 2.5, 1.4),
    },
];

pub const WALLS: [StaticBox; 3] = [
    StaticBox {
        center: Vec3::new(1.0, 1.0, 8.0),
        size: Vec3::new(6.0, 2.0, 0.5),
    },
    StaticBox {
        center: Vec3::new(12.0, 1.0, 4.0),
        size: Vec3::new(0.5, 2.0, 6.0),
    },
    StaticBox {
        center: Vec3::new(15.0, 1.0, 7.0),
        size: Vec3::new(6.0, 2.0, 0.5),
    },
];

pub fn static_colliders() -> Vec<Aabb> {
    std::iter::once(GROUND)
        .chain(PLATFORMS)
        .chain(WALLS)
        .map(StaticBox::bounds)
        .collect()
}
