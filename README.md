# Futurx

- Rust + iced를 활용한 지표 자동 매매 프로그램
- 실시간 차트를 위해 저장모드 gui사용 


## 활용지표

### 1.knn
### 구조

src/
├── main.rs
├── api/
│   ├── mod.rs
│   ├── binance.rs
│   └── exchange_info.rs
├── ui/
│   ├── mod.rs
│   ├── chart.rs
│   ├── widgets.rs
│   └── styles.rs
├── trading/
│   ├── mod.rs
│   ├── strategy.rs
│   ├── indicators.rs
│   └── execution.rs
├── models/
│   ├── mod.rs
│   ├── account.rs
│   └── market.rs
└── utils/
    ├── mod.rs
    └── helpers.rs


## 
심볼별 거래 정밀도 조회
최소 거래 금액 체크
거래 수량/가격 정밀도 조정
거래소 규칙 검증

## models

models/mod.rs:

모든 구조체 public export

models/account.rs:

AccountInfo
Position
Balance
Trade 구조체
계정 관련 데이터 구조

models/market.rs:

Candlestick
CandleType
CoinInfo
AlertType
Alert
시장 데이터 관련 구조체