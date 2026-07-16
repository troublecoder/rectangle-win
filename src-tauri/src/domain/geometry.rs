use euclid::{Point2D, Rect, Size2D, Vector2D};

/// 픽셀 단위 마커 (euclid unit type).
/// euclid 0.22은 모든 기하 타입에 단위(unit) 제네릭을 요구하므로
/// 화면 픽셀 좌표계를 표현하는 단위 타입을 정의한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pixel;

/// 픽셀 좌표계 (물리적 픽셀, Win32 좌표계와 일치)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorBounds {
    pub origin: Point2D<i32, Pixel>,
    pub size: Size2D<i32, Pixel>,
}

impl MonitorBounds {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            origin: Point2D::new(x, y),
            size: Size2D::new(width, height),
        }
    }

    pub fn center(&self) -> Point2D<i32, Pixel> {
        Point2D::new(
            self.origin.x + self.size.width / 2,
            self.origin.y + self.size.height / 2,
        )
    }

    pub fn width(&self) -> i32 {
        self.size.width
    }

    pub fn height(&self) -> i32 {
        self.size.height
    }
}

/// 커서 이동 델타(시작점 기준)로부터 섹터 인덱스 산출.
/// sector_count: 4, 8, 12 중 하나.
/// 반환값: 0..sector_count 범위의 섹터 인덱스.
/// 섹터 배치 (8섹터 기준, 시계방향, 0=오른쪽):
///   0=오른쪽, 1=오른쪽아래, 2=아래, 3=왼쪽아래,
///   4=왼쪽, 5=왼쪽위, 6=위, 7=오른쪽위
pub fn compute_sector(delta: Vector2D<f64, Pixel>, sector_count: u8) -> u8 {
    // 화면 좌표계(y축 아래가 양수)에서 atan2(y, x)는
    // 0=오른쪽, y>0(아래)=시계방향 양수 각도를 갖는다.
    // angle 범위: [-PI, PI]. 이를 [0, 2PI)로 정규화.
    let angle = delta.y.atan2(delta.x);
    let angle = if angle < 0.0 { angle + std::f64::consts::TAU } else { angle };
    let sector_size = std::f64::consts::TAU / sector_count as f64;
    // 각도를 섹터 인덱스로 변환 (반올림으로 경계 처리)
    let index = ((angle + sector_size / 2.0) / sector_size).floor() as u8;
    index % sector_count
}

/// SnapTarget의 비율 좌표를 모니터의 픽셀 Rect로 변환.
///
/// `round()` 를 사용하여 반올림 — `as i32`(버림)는 0.333*1920=639.36→639 처럼
/// 누적 손실이 발생하여 snap 영역에 의도치 않은 마진이 생긴다.
pub fn ratio_to_pixels(
    x_ratio: f64,
    y_ratio: f64,
    w_ratio: f64,
    h_ratio: f64,
    monitor: &MonitorBounds,
) -> Rect<i32, Pixel> {
    Rect::new(
        Point2D::new(
            monitor.origin.x + (x_ratio * monitor.width() as f64).round() as i32,
            monitor.origin.y + (y_ratio * monitor.height() as f64).round() as i32,
        ),
        Size2D::new(
            (w_ratio * monitor.width() as f64).round() as i32,
            (h_ratio * monitor.height() as f64).round() as i32,
        ),
    )
}

/// 델타의 거리(픽셀) 계산 — Long Throw 임계값 판별용
pub fn throw_distance(delta: Vector2D<f64, Pixel>) -> f64 {
    (delta.x * delta.x + delta.y * delta.y).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor_1080p() -> MonitorBounds {
        MonitorBounds::new(0, 0, 1920, 1080)
    }

    // ─── compute_sector 테스트 ───

    #[test]
    fn sector_right() {
        let delta = Vector2D::new(100.0, 0.0); // 오른쪽
        assert_eq!(compute_sector(delta, 8), 0);
    }

    #[test]
    fn sector_down() {
        let delta = Vector2D::new(0.0, 100.0); // 아래 (화면 좌표계 y+)
        assert_eq!(compute_sector(delta, 8), 2);
    }

    #[test]
    fn sector_left() {
        let delta = Vector2D::new(-100.0, 0.0); // 왼쪽
        assert_eq!(compute_sector(delta, 8), 4);
    }

    #[test]
    fn sector_up() {
        let delta = Vector2D::new(0.0, -100.0); // 위 (화면 좌표계 y-)
        assert_eq!(compute_sector(delta, 8), 6);
    }

    #[test]
    fn sector_down_right_diagonal() {
        let delta = Vector2D::new(100.0, 100.0); // 오른쪽 아래 대각선
        assert_eq!(compute_sector(delta, 8), 1);
    }

    #[test]
    fn sector_up_left_diagonal() {
        let delta = Vector2D::new(-100.0, -100.0); // 왼쪽 위 대각선
        assert_eq!(compute_sector(delta, 8), 5);
    }

    #[test]
    fn sector_4_count() {
        let delta = Vector2D::new(100.0, 0.0); // 오른쪽
        assert_eq!(compute_sector(delta, 4), 0);
        let delta = Vector2D::new(0.0, 100.0); // 아래
        assert_eq!(compute_sector(delta, 4), 1);
    }

    #[test]
    fn sector_zero_delta() {
        let delta = Vector2D::new(0.0, 0.0); // 델타 없음
        // 0섹터(오른쪽)로 폴백
        let result = compute_sector(delta, 8);
        assert!(result < 8);
    }

    // ─── ratio_to_pixels 테스트 ───

    #[test]
    fn ratio_left_half_to_pixels() {
        let monitor = monitor_1080p();
        let rect = ratio_to_pixels(0.0, 0.0, 0.5, 1.0, &monitor);
        assert_eq!(rect.origin, Point2D::new(0, 0));
        assert_eq!(rect.size, Size2D::new(960, 1080));
    }

    #[test]
    fn ratio_right_half_to_pixels() {
        let monitor = monitor_1080p();
        let rect = ratio_to_pixels(0.5, 0.0, 0.5, 1.0, &monitor);
        assert_eq!(rect.origin, Point2D::new(960, 0));
        assert_eq!(rect.size, Size2D::new(960, 1080));
    }

    #[test]
    fn ratio_center_to_pixels() {
        let monitor = monitor_1080p();
        let rect = ratio_to_pixels(0.25, 0.25, 0.5, 0.5, &monitor);
        assert_eq!(rect.origin, Point2D::new(480, 270));
        assert_eq!(rect.size, Size2D::new(960, 540));
    }

    #[test]
    fn ratio_with_monitor_offset() {
        let monitor = MonitorBounds::new(1920, 0, 1920, 1080); // 두 번째 모니터
        let rect = ratio_to_pixels(0.0, 0.0, 0.5, 1.0, &monitor);
        assert_eq!(rect.origin, Point2D::new(1920, 0));
    }

    // ─── throw_distance 테스트 ───

    #[test]
    fn throw_distance_simple() {
        let delta = Vector2D::new(300.0, 400.0); // 3-4-5 삼각비
        assert!((throw_distance(delta) - 500.0).abs() < 0.001);
    }

    #[test]
    fn throw_distance_zero() {
        assert!((throw_distance(Vector2D::new(0.0, 0.0))).abs() < 0.001);
    }

    // ─── MonitorBounds 테스트 ───

    #[test]
    fn monitor_center() {
        let monitor = monitor_1080p();
        assert_eq!(monitor.center(), Point2D::new(960, 540));
    }
}
