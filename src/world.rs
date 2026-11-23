use bfge::math::Vec3;

use crate::zone;

pub const START_POSITION: Vec3 = Vec3::new(-4.0, 0.0, 2.0);
pub const TIMING_START_POSITION: Vec3 = Vec3::new(-4.0, 0.0, -6.0);

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

    pub fn overlaps(self, other: Aabb) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
            && self.min.z < other.max.z
            && self.max.z > other.min.z
    }
}

pub const TUTORIAL_EVENT_ZONE: Aabb = Aabb {
    min: Vec3::new(-8.0, 0.0, 0.5),
    max: Vec3::new(-6.0, 2.0, 3.5),
};

pub const CHECKPOINT_EVENT_ZONE: Aabb = Aabb {
    min: Vec3::new(-19.0, 1.5, 1.0),
    max: Vec3::new(-17.0, 3.5, 3.0),
};

pub fn event_zones() -> [zone::Zone; 2] {
    [
        zone::Zone::repeating(TUTORIAL_EVENT_ZONE),
        zone::Zone::one_shot(CHECKPOINT_EVENT_ZONE),
    ]
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
    center: Vec3::new(55.0, -0.1, 0.0),
    size: Vec3::new(130.0, 0.2, 200.0),
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

pub const WALLS: [StaticBox; 4] = [
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
    // Параллельная стена оставляет проход шириной 1,5 м.
    StaticBox {
        center: Vec3::new(14.0, 1.25, 4.0),
        size: Vec3::new(0.5, 2.5, 6.0),
    },
];

/// Уступы для спуска, перекладина для удара головой и две опоры с разрывом.
pub const VERTICAL_COURSE: [StaticBox; 6] = [
    StaticBox {
        center: Vec3::new(-4.0, 1.5, -14.0),
        size: Vec3::new(5.0, 3.0, 5.0),
    },
    StaticBox {
        center: Vec3::new(1.0, 1.0, -14.0),
        size: Vec3::new(5.0, 2.0, 5.0),
    },
    StaticBox {
        center: Vec3::new(6.0, 0.5, -14.0),
        size: Vec3::new(5.0, 1.0, 5.0),
    },
    StaticBox {
        center: Vec3::new(-5.0, 2.8, -24.0),
        size: Vec3::new(6.0, 0.3, 5.0),
    },
    StaticBox {
        center: Vec3::new(9.0, 0.5, -24.0),
        size: Vec3::new(4.0, 1.0, 5.0),
    },
    StaticBox {
        center: Vec3::new(15.0, 0.5, -24.0),
        size: Vec3::new(4.0, 1.0, 5.0),
    },
];

pub const LOWER_TERRACE: StaticBox = StaticBox {
    center: Vec3::new(101.5, -1.25, 2.0),
    size: Vec3::new(220.0, 2.5, 76.0),
};

pub const UPPER_TERRACE: StaticBox = StaticBox {
    center: Vec3::new(-139.0, 1.25, -4.8),
    size: Vec3::new(220.0, 5.0, 76.0),
};

/// С каждой опорой маршрут поднимается на 0,75 м.
pub const ASCENT: [StaticBox; 4] = [
    StaticBox {
        center: Vec3::new(-12.6, -0.25, 2.0),
        size: Vec3::new(4.0, 2.0, 4.6),
    },
    StaticBox {
        center: Vec3::new(-18.0, 0.50, 2.0),
        size: Vec3::new(4.4, 2.0, 4.6),
    },
    StaticBox {
        center: Vec3::new(-18.5, 1.25, -4.8),
        size: Vec3::new(4.2, 2.0, 4.0),
    },
    StaticBox {
        center: Vec3::new(-25.0, 2.00, -4.8),
        size: Vec3::new(4.4, 2.0, 4.0),
    },
];

pub const COURSE_WALLS: [StaticBox; 3] = [
    // Закрывает прямой путь и оставляет боковой выход со второй опоры.
    StaticBox {
        center: Vec3::new(-20.0, 2.9, 2.0),
        size: Vec3::new(0.4, 2.8, 4.6),
    },
    // Вместе с торцевой стеной образует угол для проверки камеры.
    StaticBox {
        center: Vec3::new(-18.55, 2.9, 4.1),
        size: Vec3::new(3.3, 2.8, 0.4),
    },
    // Скрывает боковой разрыв и следующую опору со стартовой дорожки.
    StaticBox {
        center: Vec3::new(-14.75, 1.50, -3.725),
        size: Vec3::new(1.1, 6.0, 6.95),
    },
];

pub fn diagnostic_blocks() -> impl Iterator<Item = StaticBox> {
    PLATFORMS.into_iter().chain(WALLS).chain(VERTICAL_COURSE)
}

pub fn course_colliders() -> Vec<Aabb> {
    [LOWER_TERRACE, UPPER_TERRACE]
        .into_iter()
        .chain(ASCENT)
        .chain(COURSE_WALLS)
        .map(StaticBox::bounds)
        .collect()
}

pub fn diagnostic_colliders() -> Vec<Aabb> {
    std::iter::once(GROUND)
        .chain(diagnostic_blocks())
        .map(StaticBox::bounds)
        .collect()
}
