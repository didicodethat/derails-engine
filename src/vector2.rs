use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::mem::swap;

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Debug, Clone, Copy, JsonSchema)]
pub struct Vector2(pub i32, pub i32);

impl Vector2 {
    pub fn len(&self) -> f32 {
        f32::sqrt(((self.0 * self.0) + (self.1 * self.1)) as f32)
    }
}

impl std::fmt::Display for Vector2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vector2({}, {})", self.0, self.1)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Vector2Range {
    x: i32,
    y: i32,
    x2: i32,
    yaxis: bool,
    w: i32,
    h: i32,
    dx: i32,
    dy: i32,
    e: i32,
}

impl Vector2Range {
    pub fn new(from: &Vector2, to: &Vector2) -> Self {
        let mut x1 = from.0;
        let mut y1 = from.1;
        let mut y2 = to.1;
        let mut tmp = Self {
            x2: to.0,
            yaxis: i32::abs(to.1 - from.1) > i32::abs(to.0 - from.0),
            w: 0,
            h: 0,
            dx: 0,
            dy: 0,
            y: 0,
            x: 0,
            e: 0,
        };

        if tmp.yaxis {
            swap(&mut y1, &mut x1);
            swap(&mut y2, &mut tmp.x2);
            // let a = y1;
            // x1 = a;
            // let b = y2;
            // y2 = tmp.x2;
            // tmp.x2 = b;
        }
        tmp.w = i32::abs(tmp.x2 - x1) + 1;
        tmp.h = i32::abs(y2 - y1) + 1;
        tmp.dx = i32::signum(tmp.x2 - x1);
        tmp.dy = i32::signum(y2 - y1);
        tmp.y = y1;
        tmp.x = x1;
        tmp.x2 += tmp.dx;

        tmp
    }
}

impl Iterator for Vector2Range {
    type Item = Vector2;
    fn next(&mut self) -> Option<Self::Item> {
        if self.x != self.x2 {
            let res = Some(if self.yaxis {
                Vector2(self.y, self.x)
            } else {
                Vector2(self.x, self.y)
            });
            self.e += self.h;
            if self.e >= self.w {
                self.y += self.dy;
                self.e -= self.w;
            }
            self.x += self.dx;
            res
        } else {
            None
        }
    }
}
