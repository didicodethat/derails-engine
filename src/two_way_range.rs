pub struct TwoWayRange{
    start: i32,
    end: i32,
    step: i32,
    current: i32
}
#[derive(Debug)]
pub enum TwoWayRangeError {
    ZeroStep
}

impl TwoWayRange {
    pub fn new(start: i32, end: i32, step: i32) -> Result<Self,TwoWayRangeError> {
        if step == 0 {
            return Err(TwoWayRangeError::ZeroStep);
        }
        Ok(Self {
            start, 
            end,
            step,
            current: start
        })
    }
    #[inline]
    fn do_step(&mut self) -> i32 {
        let before = self.current;
        self.current += self.step;
        return before;
    }
}

impl Iterator for TwoWayRange {
    type Item = i32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            if self.current > self.end {
                Some(self.do_step())
            } else {
                None
            }
        } else {
            if self.current < self.end {
                Some(self.do_step())
            } else {
                None
            }
        }
    }
}