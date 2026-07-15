use std::time::{Duration, Instant};

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::model::Direction;

/// 키보드 체인 순환 상태 추적.
/// 같은 방향 + 같은 창 + 타임아웃 내 연속 탭 → 다음 인덱스로 진행.
/// 아니면 인덱스 0부터 재시작.
#[derive(Debug, Clone)]
pub struct ChainCycle {
    last_direction: Option<Direction>,
    last_index: usize,
    last_window: u64, // 윈도우 핸들 (Win32 HWND를 u64로 표현)
    last_time: Option<Instant>,
    timeout: Duration,
}

impl ChainCycle {
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            last_direction: None,
            last_index: 0,
            last_window: 0,
            last_time: None,
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    /// 체인에서 다음 타겟의 인덱스를 반환.
    /// direction: 눌린 방향키
    /// window_handle: 현재 포커스된 윈도우 핸들
    /// chain: 해당 방향의 체인 (horizontal 또는 vertical)
    pub fn next_index(
        &mut self,
        direction: Direction,
        window_handle: u64,
        chain: &[String],
    ) -> DomainResult<usize> {
        if chain.is_empty() {
            return Err(DomainError::EmptyChain);
        }

        let now = Instant::now();
        let should_advance = self.is_continuous(direction, window_handle, now);

        let new_index = if should_advance {
            // 정방향(Right, Down): 인덱스 증가
            // 역방향(Left, Up): 인덱스 감소
            if direction.is_forward() {
                (self.last_index + 1) % chain.len()
            } else {
                if self.last_index == 0 {
                    chain.len() - 1 // 끝에서 처음으로 (순환)
                } else {
                    self.last_index - 1
                }
            }
        } else {
            // 새 시퀀스 시작 — 항상 인덱스 0
            0
        };

        self.last_direction = Some(direction);
        self.last_index = new_index;
        self.last_window = window_handle;
        self.last_time = Some(now);

        Ok(new_index)
    }

    /// 연속 탭 조건: 같은 축(가로/세로) + 같은 창 + 타임아웃 내
    fn is_continuous(&self, direction: Direction, window_handle: u64, now: Instant) -> bool {
        match (self.last_direction, self.last_time) {
            (Some(last_dir), Some(last_time)) => {
                let same_axis = if direction.is_horizontal() {
                    last_dir.is_horizontal()
                } else if direction.is_vertical() {
                    last_dir.is_vertical()
                } else {
                    false
                };
                let same_window = self.last_window == window_handle;
                let within_timeout = now.duration_since(last_time) < self.timeout;
                same_axis && same_window && within_timeout
            }
            _ => false,
        }
    }

    /// 상태 초기화 (창 포커스 변경 등)
    pub fn reset(&mut self) {
        self.last_direction = None;
        self.last_index = 0;
        self.last_window = 0;
        self.last_time = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h_chain() -> Vec<String> {
        vec![
            "left-half".to_string(),
            "third-left".to_string(),
            "center".to_string(),
            "third-right".to_string(),
            "right-half".to_string(),
        ]
    }

    fn v_chain() -> Vec<String> {
        vec![
            "maximize".to_string(),
            "almost-maximize".to_string(),
            "center".to_string(),
            "maximize-height".to_string(),
        ]
    }

    const WINDOW: u64 = 12345;

    #[test]
    fn first_tap_starts_at_index_zero() {
        let mut cycle = ChainCycle::new(1500);
        let idx = cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap();
        assert_eq!(idx, 0); // 첫 탭 = "left-half"
    }

    #[test]
    fn forward_advances_chain() {
        let mut cycle = ChainCycle::new(1500);
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 0
        let idx = cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 1
        assert_eq!(idx, 1); // "third-left"
    }

    #[test]
    fn forward_wraps_around() {
        let mut cycle = ChainCycle::new(1500);
        // 끝까지 진행
        for _ in 0..5 {
            cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap();
        }
        // 한 번 더 → 처음으로 순환
        let idx = cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap();
        assert_eq!(idx, 0);
    }

    #[test]
    fn backward_decrements() {
        let mut cycle = ChainCycle::new(1500);
        // 먼저 정방향으로 인덱스 2까지 진행
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 0
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 1
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 2

        // 역방향 (Left)
        let idx = cycle.next_index(Direction::Left, WINDOW, &h_chain()).unwrap();
        assert_eq!(idx, 1); // 2에서 1로 감소
    }

    #[test]
    fn backward_wraps_from_zero_to_last() {
        let mut cycle = ChainCycle::new(1500);
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 0

        // 인덱스 0에서 Left → 마지막으로 순환
        let idx = cycle.next_index(Direction::Left, WINDOW, &h_chain()).unwrap();
        assert_eq!(idx, 4); // "right-half"
    }

    #[test]
    fn different_window_resets_to_zero() {
        let mut cycle = ChainCycle::new(1500);
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 0
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 1

        // 다른 창
        let idx = cycle.next_index(Direction::Right, 99999, &h_chain()).unwrap();
        assert_eq!(idx, 0);
    }

    #[test]
    fn different_axis_resets_to_zero() {
        let mut cycle = ChainCycle::new(1500);
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 0
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap(); // 1

        // 세로 체인으로 전환
        let idx = cycle.next_index(Direction::Down, WINDOW, &v_chain()).unwrap();
        assert_eq!(idx, 0); // 다른 축이므로 리셋
    }

    #[test]
    fn vertical_chain_forward() {
        let mut cycle = ChainCycle::new(1500);
        cycle.next_index(Direction::Down, WINDOW, &v_chain()).unwrap(); // 0: maximize
        let idx = cycle.next_index(Direction::Down, WINDOW, &v_chain()).unwrap();
        assert_eq!(idx, 1); // almost-maximize
    }

    #[test]
    fn vertical_chain_backward() {
        let mut cycle = ChainCycle::new(1500);
        cycle.next_index(Direction::Down, WINDOW, &v_chain()).unwrap(); // 0
        cycle.next_index(Direction::Down, WINDOW, &v_chain()).unwrap(); // 1

        let idx = cycle.next_index(Direction::Up, WINDOW, &v_chain()).unwrap();
        assert_eq!(idx, 0); // 1에서 0으로 역행
    }

    #[test]
    fn empty_chain_returns_error() {
        let mut cycle = ChainCycle::new(1500);
        let result = cycle.next_index(Direction::Right, WINDOW, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn reset_clears_state() {
        let mut cycle = ChainCycle::new(1500);
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap();
        cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap();
        assert_eq!(cycle.last_index, 1);

        cycle.reset();
        let idx = cycle.next_index(Direction::Right, WINDOW, &h_chain()).unwrap();
        assert_eq!(idx, 0);
    }
}
