use crate::ratio::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    Int(isize),
    Float(f64),
    Rational(Ratio<isize>),
}

impl Number {
    pub fn plus(self, rhs: Number) -> Number {
        match self {
            Number::Int(i) => match rhs {
                Number::Int(j) => Number::Int(i + j),
                Number::Float(j) => Number::Float(i as f64 + j),
                Number::Rational(j) => Number::Rational(Ratio::from(i) + j),
            },
            Number::Float(i) => match rhs {
                Number::Int(j) => Number::Float(i + j as f64),
                Number::Float(j) => Number::Float(i + j),
                Number::Rational(j) => Number::Float(i + j.to_f64()),
            },
            Number::Rational(i) => match rhs {
                Number::Int(j) => Number::Rational(i + Ratio::from(j)),
                Number::Float(j) => Number::Float(i.to_f64() + j),
                Number::Rational(j) => Number::Rational(i + j),
            },
        }
    }

    pub fn minus(self, rhs: Number) -> Number {
        match self {
            Number::Int(i) => match rhs {
                Number::Int(j) => Number::Int(i - j),
                Number::Float(j) => Number::Float(i as f64 - j),
                Number::Rational(j) => Number::Rational(Ratio::from(i) - j),
            },
            Number::Float(i) => match rhs {
                Number::Int(j) => Number::Float(i - j as f64),
                Number::Float(j) => Number::Float(i - j),
                Number::Rational(j) => Number::Float(i - j.to_f64()),
            },
            Number::Rational(i) => match rhs {
                Number::Int(j) => Number::Rational(i - Ratio::from(j)),
                Number::Float(j) => Number::Float(i.to_f64() - j),
                Number::Rational(j) => Number::Rational(i - j),
            },
        }
    }

    pub fn times(self, rhs: Number) -> Number {
        match self {
            Number::Int(i) => match rhs {
                Number::Int(j) => Number::Int(i * j),
                Number::Float(j) => Number::Float(i as f64 * j),
                Number::Rational(j) => Number::Rational(Ratio::from(i) * j),
            },
            Number::Float(i) => match rhs {
                Number::Int(j) => Number::Float(i * j as f64),
                Number::Float(j) => Number::Float(i * j),
                Number::Rational(j) => Number::Float(i * j.to_f64()),
            },
            Number::Rational(i) => match rhs {
                Number::Int(j) => Number::Rational(i * Ratio::from(j)),
                Number::Float(j) => Number::Float(i.to_f64() * j),
                Number::Rational(j) => Number::Rational(i * j),
            },
        }
    }
    
    pub fn div(self, rhs: Number) -> Number {
        match self {
            Number::Int(i) => match rhs {
                Number::Int(j) => Number::Int(i / j),
                Number::Float(j) => Number::Float(i as f64 / j),
                Number::Rational(j) => Number::Rational(Ratio::from(i) / j),
            },
            Number::Float(i) => match rhs {
                Number::Int(j) => Number::Float(i / j as f64),
                Number::Float(j) => Number::Float(i / j),
                Number::Rational(j) => Number::Float(i / j.to_f64()),
            },
            Number::Rational(i) => match rhs {
                Number::Int(j) => Number::Rational(i / Ratio::from(j)),
                Number::Float(j) => Number::Float(i.to_f64() / j),
                Number::Rational(j) => Number::Rational(i / j),
            },
        }
    }

    pub fn eq(self, rhs: Number) -> bool {
        match self {
            Number::Int(i) => match rhs {
                Number::Int(j) => i == j,
                Number::Float(j) => i as f64 == j,
                Number::Rational(j) => Ratio::from(i) == j,
            },
            Number::Float(i) => match rhs {
                Number::Int(j) => i == j as f64,
                Number::Float(j) => i == j,
                Number::Rational(j) => i == j.to_f64(),
            },
            Number::Rational(i) => match rhs {
                Number::Int(j) => i == Ratio::from(j),
                Number::Float(j) => i.to_f64() == j,
                Number::Rational(j) => i == j,
            },
        }
    }

    pub fn ge(self, rhs: Number) -> bool {
        match self {
            Number::Int(i) => match rhs {
                Number::Int(j) => i >= j,
                Number::Float(j) => i as f64 >= j,
                Number::Rational(j) => Ratio::from(i) >= j,
            },
            Number::Float(i) => match rhs {
                Number::Int(j) => i >= j as f64,
                Number::Float(j) => i >= j,
                Number::Rational(j) => i >= j.to_f64(),
            },
            Number::Rational(i) => match rhs {
                Number::Int(j) => i >= Ratio::from(j),
                Number::Float(j) => i.to_f64() >= j,
                Number::Rational(j) => i >= j,
            },
        }
    }

    pub fn gt(self, rhs: Number) -> bool {
        match self {
            Number::Int(i) => match rhs {
                Number::Int(j) => i > j,
                Number::Float(j) => i as f64 > j,
                Number::Rational(j) => Ratio::from(i) > j,
            },
            Number::Float(i) => match rhs {
                Number::Int(j) => i > j as f64,
                Number::Float(j) => i > j,
                Number::Rational(j) => i > j.to_f64(),
            },
            Number::Rational(i) => match rhs {
                Number::Int(j) => i > Ratio::from(j),
                Number::Float(j) => i.to_f64() > j,
                Number::Rational(j) => i > j,
            },
        }
    }

    pub fn lt(self, rhs: Number) -> bool {
        !self.ge(rhs)
    }

    pub fn le(self, rhs: Number) -> bool {
        !self.gt(rhs)
    }

    pub fn to_isize(self) -> isize {
        match self {
            Number::Int(i) => i,
            Number::Float(i) => i as isize,
            Number::Rational(r) => r.to_f64() as isize,
        }
    }

    pub fn ceiling(self) -> Number {
        match self {
            Number::Int(i) => Number::Int(i),
            Number::Float(i) => Number::Int(i.ceil() as isize),
            Number::Rational(r) => Number::Int(r.to_f64().ceil() as isize),
        }
    }

    pub fn floor(self) -> Number {
        match self {
            Number::Int(i) => Number::Int(i),
            Number::Float(i) => Number::Int(i.floor() as isize),
            Number::Rational(r) => Number::Int(r.to_f64().floor() as isize),
        }
    }

    pub fn sqrt(self) -> Number {
        match self {
            Number::Int(i) => Number::Int((i as f64).sqrt().floor() as isize),
            Number::Float(i) => Number::Float(i.sqrt()),
            Number::Rational(r) => Number::Rational(
                Ratio::new(
                    ((r.numerator as f64).sqrt() as isize).into(),
                    ((r.denominator as f64).sqrt() as isize).into(),
                ),
            ),
        }
    }

    pub fn modulus(self, rhs: Number) -> Number {
        let a = self.to_isize();
        let b = rhs.to_isize();
        Number::Int(a % b)
    }
}
