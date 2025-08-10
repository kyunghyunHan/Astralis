# Asterism
- 암호화폐 자동매매 시스템 개발 프로젝트

## 프로젝트 개요

Rust 기반의 데스크톱 애플리케이션으로, Binance API를 활용한 실시간 암호화폐 자동매매 시스템을 개발했습니다. GUI 프레임워크로 Iced를 사용하여 사용자 친화적인 인터페이스를 구현했으며, 다양한 기술적 지표와 머신러닝 알고리즘을 결합하여 매매 신호를 생성합니다.

## 주요 기술 스택

- **언어 및 프레임워크**: 
  - Rust 
  - Iced (GUI 프레임워크)
  - Tokio (비동기 런타임)

- **외부 API 및 서비스**:
  - Binance Futures API
  - WebSocket 실시간 데이터 통신

## 핵심 기능 구현

### 1. 데이터 시각화 및 차트 시스템
- 실시간 캔들스틱 차트 구현
- 다중 시간대 지원 (1분봉, 3분봉, 일봉)
- 기술적 지표 오버레이:
  - 이동평균선 (MA 5, 10, 20, 200)
  - 거래량 지표
  - KNN 기반 매매 신호

### 2. 머신러닝 기반 매매 시스템
- KNN(K-Nearest Neighbors) 알고리즘 구현
- 최적화된 특성 추출:
  ```rust
  pub fn extract_features(&self, candlesticks: &[(&u64, &Candlestick)]) -> Option<Vec<f32>> {
      let mut features = Vec::with_capacity(self.window_size * 4);
      // 가격 변화율, 이동평균, RSI, 거래량 비율 계산
      features.extend_from_slice(&[
          ma5 / ma20 - 1.0,             // MA 비율
          rsi / 100.0,                  // 정규화된 RSI
          volume_ratio,                 // 거래량 비율
          price_changes.last().unwrap_or(&0.0) / 100.0
      ]);
      Some(features)
  }
  ```

### 3. 실시간 거래 실행 시스템
- 비동기 주문 처리 시스템 구현
- 포지션 관리 및 리스크 관리
- 시장가 매수/매도 기능:
  ```rust
  pub fn market_buy(r: &mut Futurx) {
      if let Some(info) = r.coin_list.get(&r.selected_coin) {
          if let Some(account_info) = &r.account_info {
              // 포지션 크기 계산 및 주문 실행
              let total_quantity = uc::MARKET_BUY_ORDER_PRICE / price;
              // 비동기 주문 실행
          }
      }
  }
  ```

### 4. 상태 관리 및 UI 시스템
- Iced 프레임워크를 활용한 반응형 UI 구현
- 실시간 가격 정보 및 포지션 정보 표시
- 거래 알림 시스템:
  ```rust
  fn add_alert(&mut self, message: String, alert_type: AlertType) {
      self.alerts.push_back(Alert {
          message,
          alert_type,
          timestamp: Instant::now(),
      });
  }
  ```

## 기술적 도전과 해결 방안

### 1. 실시간 데이터 처리
- **도전**: WebSocket을 통한 대량의 실시간 데이터 처리
- **해결**: 
  - Tokio 비동기 런타임 활용
  - 효율적인 메모리 관리를 위한 자료구조 최적화
  - 데이터 버퍼링 시스템 구현

### 2. 성능 최적화
- **도전**: 복잡한 차트 렌더링과 실시간 데이터 업데이트
- **해결**:
  - 캔들스틱 데이터 효율적 관리를 위한 BTreeMap 사용
  - 렌더링 최적화를 위한 캐싱 시스템 구현

### 3. 안정적인 거래 실행
- **도전**: 네트워크 지연 및 에러 처리
- **해결**:
  - 견고한 에러 처리 시스템 구현
  - 재시도 메커니즘 도입
  - 거래 타임아웃 설정


## 시사점

- Rust의 안전성과 성능을 활용한 금융 거래 시스템 구현 경험
- 비동기 프로그래밍과 실시간 데이터 처리 능력 향상
- GUI 애플리케이션 개발 및 사용자 경험 최적화 경험