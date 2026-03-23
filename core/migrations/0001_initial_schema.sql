-- ==============================================================================
-- 0. CÁC KIỂU DỮ LIỆU CƠ BẢN (ENUM)
-- Định nghĩa trước các trạng thái cố định để tiết kiệm dung lượng lưu trữ
-- ==============================================================================
CREATE TYPE order_side   AS ENUM ('buy', 'sell'); -- Hướng lệnh: Mua hoặc Bán
CREATE TYPE order_status AS ENUM ('open', 'partial', 'filled', 'cancelled'); 
-- open: Đang chờ khớp
-- partial: Khớp một phần
-- filled: Đã khớp hết
-- cancelled: Đã hủy

-- ==============================================================================
-- 1. BẢNG `users` (NGƯỜI DÙNG)
-- Lưu trữ thông tin cơ bản của người dùng trên sàn.
-- ==============================================================================
CREATE TABLE IF NOT EXISTS users (
    id            BIGINT      PRIMARY KEY,           -- ID người dùng (lấy từ Header của API)
    username      TEXT        NOT NULL UNIQUE,       -- Tên đăng nhập (duy nhất)
    display_name  TEXT        NOT NULL,              -- Tên hiển thị (có thể thay đổi)
    password_hash TEXT,                              -- Mật khẩu đã mã hóa bằng Argon2id
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now() -- Thời gian tạo tài khoản
);

-- ==============================================================================
-- 2. BẢNG `assets` (TÀI SẢN / ĐỒNG COIN)
-- Danh sách các loại tiền điện tử hoặc tiền thật được phép giao dịch trên sàn.
-- ==============================================================================
CREATE TABLE IF NOT EXISTS assets (
    symbol     TEXT        PRIMARY KEY,           -- Mã tài sản (VD: 'BTC', 'USDT')
    name       TEXT        NOT NULL,              -- Tên đầy đủ (VD: 'Bitcoin', 'Tether USD')
    decimals   SMALLINT    NOT NULL DEFAULT 8,    -- Số chữ số thập phân (VD: BTC là 8, USDT là 2)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ==============================================================================
-- 3. BẢNG `markets` (CẶP GIAO DỊCH)
-- Quản lý các cặp tiền được phép giao dịch với nhau, tránh tình trạng user gõ sai.
-- ==============================================================================
CREATE TABLE IF NOT EXISTS markets (
    symbol      TEXT        PRIMARY KEY,           -- Mã cặp giao dịch (VD: 'BTC_USDT')
    base_asset  TEXT        NOT NULL REFERENCES assets(symbol),  -- Đồng tiền cơ sở (VD: BTC)
    quote_asset TEXT        NOT NULL REFERENCES assets(symbol),  -- Đồng tiền định giá (VD: USDT)
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    -- Ràng buộc: Không thể tạo cặp giao dịch trùng nhau (VD: BTC_BTC)
    CONSTRAINT chk_different_assets CHECK (base_asset != quote_asset) 
);

-- ==============================================================================
-- 4. BẢNG `balances` (SỐ DƯ TÀI KHOẢN)
-- Quản lý ví tiền của người dùng cho từng loại tài sản.
-- ==============================================================================
CREATE TABLE IF NOT EXISTS balances (
    user_id      BIGINT        NOT NULL REFERENCES users(id),       -- ID người sở hữu
    asset_symbol TEXT          NOT NULL REFERENCES assets(symbol),  -- Mã tài sản (BTC, USDT)
    available    NUMERIC(30,8) NOT NULL DEFAULT 0,                  -- Số dư khả dụng (có thể rút/giao dịch)
    locked       NUMERIC(30,8) NOT NULL DEFAULT 0,                  -- Số dư bị khóa (đang nằm trong lệnh chờ khớp)
    updated_at   TIMESTAMPTZ   NOT NULL DEFAULT now(),              -- Lần cập nhật cuối

    PRIMARY KEY (user_id, asset_symbol), -- Một user chỉ có 1 ví cho mỗi loại tài sản

    -- Ràng buộc: Số dư không bao giờ được phép âm
    CONSTRAINT chk_available_ge_zero CHECK (available >= 0),
    CONSTRAINT chk_locked_ge_zero    CHECK (locked    >= 0)
);

-- ==============================================================================
-- 5. BẢNG `orders_log` (NHẬT KÝ ĐẶT LỆNH)
-- Lưu lại toàn bộ lịch sử đặt lệnh của người dùng từ hệ thống Matching Engine.
-- ==============================================================================
CREATE TABLE IF NOT EXISTS orders_log (
    id            UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id      BIGINT        NOT NULL UNIQUE,                     -- ID gốc của lệnh từ Matching Engine
    user_id       BIGINT        NOT NULL REFERENCES users(id),       -- Người đặt lệnh
    market_symbol TEXT          NOT NULL REFERENCES markets(symbol), -- Giao dịch cặp nào (BTC_USDT)
    side          order_side    NOT NULL,                            -- Mua hay Bán
    price         NUMERIC(30,8) NOT NULL,                            -- Mức giá đặt
    amount        NUMERIC(30,8) NOT NULL,                            -- Số lượng đặt
    filled        NUMERIC(30,8) NOT NULL DEFAULT 0,                  -- Số lượng đã khớp thành công
    status        order_status  NOT NULL DEFAULT 'open',             -- Trạng thái hiện tại của lệnh
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ   NOT NULL DEFAULT now(),

    -- Các ràng buộc logic cơ bản để tránh lỗi hệ thống
    CONSTRAINT chk_price_positive  CHECK (price  > 0),
    CONSTRAINT chk_amount_positive CHECK (amount > 0),
    CONSTRAINT chk_filled_valid    CHECK (filled >= 0 AND filled <= amount)
);

-- ==============================================================================
-- 6. BẢNG `trades_log` (LỊCH SỬ KHỚP LỆNH / GIAO DỊCH)
-- Bảng này CỐ TÌNH LƯU DƯ THỪA dữ liệu (user_id, market_symbol) để tối ưu tốc độ 
-- đọc lịch sử giao dịch mà không cần JOIN nhiều bảng.
-- ==============================================================================
CREATE TABLE IF NOT EXISTS trades_log (
    id             UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    maker_order_id BIGINT        NOT NULL REFERENCES orders_log(order_id), -- Lệnh đặt chờ sẵn (Maker)
    taker_order_id BIGINT        NOT NULL REFERENCES orders_log(order_id), -- Lệnh khớp ngay lập tức (Taker)
    maker_user_id  BIGINT        NOT NULL REFERENCES users(id),            -- User đặt Maker
    taker_user_id  BIGINT        NOT NULL REFERENCES users(id),            -- User đặt Taker
    market_symbol  TEXT          NOT NULL REFERENCES markets(symbol),      -- Cặp giao dịch
    price          NUMERIC(30,8) NOT NULL,                                 -- Giá khớp thực tế
    amount         NUMERIC(30,8) NOT NULL,                                 -- Số lượng khớp
    executed_at    TIMESTAMPTZ   NOT NULL DEFAULT now(),                   -- Thời gian khớp lệnh

    CONSTRAINT chk_trade_price_positive  CHECK (price  > 0),
    CONSTRAINT chk_trade_amount_positive CHECK (amount > 0),
    CONSTRAINT chk_no_self_trade         CHECK (maker_user_id != taker_user_id) -- Không cho phép tự mua bán với chính mình
);

-- ==============================================================================
-- 7. BẢNG `candles` (BIỂU ĐỒ NẾN - OHLCV)
-- Lưu trữ dữ liệu tổng hợp theo thời gian (1 phút, 5 phút...) để vẽ biểu đồ TradingView.
-- ==============================================================================
CREATE TABLE IF NOT EXISTS candles (
    market_symbol TEXT          NOT NULL REFERENCES markets(symbol),
    interval      TEXT          NOT NULL,           -- Khung thời gian (VD: '1m', '5m', '1h', '1d')
    open_time     TIMESTAMPTZ   NOT NULL,           -- Thời gian mở nến (Bắt đầu chu kỳ)
    open          NUMERIC(30,8) NOT NULL,           -- Giá Mở cửa
    high          NUMERIC(30,8) NOT NULL,           -- Giá Cao nhất
    low           NUMERIC(30,8) NOT NULL,           -- Giá Thấp nhất
    close         NUMERIC(30,8) NOT NULL,           -- Giá Đóng cửa
    volume        NUMERIC(30,8) NOT NULL,           -- Tổng khối lượng giao dịch trong nến

    PRIMARY KEY (market_symbol, interval, open_time),

    CONSTRAINT chk_candle_open_positive   CHECK (open   > 0),
    CONSTRAINT chk_candle_high_ge_low     CHECK (high  >= low),
    CONSTRAINT chk_candle_volume_positive CHECK (volume > 0)
);

-- ==============================================================================
-- 8. HỆ THỐNG CHỈ MỤC (INDEXES) - BẮT BUỘC PHẢI CÓ ĐỂ CHẠY NHANH
-- ==============================================================================

-- Tốc độ cao nhất để tìm các lệnh ĐANG MỞ (Open Orders) của 1 user
CREATE INDEX idx_orders_active ON orders_log (user_id, created_at DESC) WHERE status IN ('open', 'partial');

-- Tìm toàn bộ lịch sử đặt lệnh của 1 user
CREATE INDEX idx_orders_history ON orders_log (user_id, created_at DESC);

-- Tìm lịch sử giao dịch cá nhân (Tách riêng Maker và Taker để lấy dữ liệu siêu tốc)
CREATE INDEX idx_trades_maker ON trades_log (maker_user_id, executed_at DESC);
CREATE INDEX idx_trades_taker ON trades_log (taker_user_id, executed_at DESC);

-- Tìm lịch sử giao dịch của cả thị trường (Dùng để show lên bảng Public Trades trên web)
CREATE INDEX idx_trades_market ON trades_log (market_symbol, executed_at DESC);

-- Tối ưu tốc độ lấy dữ liệu vẽ biểu đồ nến (Lấy nến mới nhất trước)
CREATE INDEX idx_candles_market_time ON candles (market_symbol, interval, open_time DESC);

-- ==============================================================================
-- 9. DỮ LIỆU MẪU (SEED DATA)
-- ==============================================================================

INSERT INTO users (id, username, display_name) VALUES
    (1, 'alice', 'alice'),
    (2, 'bob', 'bob'),
    (3, 'charlie', 'charlie'),
    (4, 'dave', 'dave')
ON CONFLICT (id) DO NOTHING;

INSERT INTO assets (symbol, name, decimals) VALUES
    ('BTC',  'Bitcoin',    8),
    ('ETH',  'Ethereum',   8),
    ('SOL',  'Solana',     8),
    ('BNB',  'BNB',        8),
    ('USDT', 'Tether USD', 2)
ON CONFLICT (symbol) DO NOTHING;

INSERT INTO markets (symbol, base_asset, quote_asset) VALUES
    ('BTC_USDT', 'BTC', 'USDT'),
    ('ETH_USDT', 'ETH', 'USDT'),
    ('SOL_USDT', 'SOL', 'USDT'),
    ('BNB_USDT', 'BNB', 'USDT')
ON CONFLICT (symbol) DO NOTHING;

INSERT INTO balances (user_id, asset_symbol, available, locked) VALUES
    (1, 'BTC',  100.00000000, 0), (1, 'USDT', 10000000.00000000, 0),
    (1, 'ETH',  500.00000000, 0), (1, 'SOL',     2000.00000000, 0), (1, 'BNB', 300.00000000, 0),
    (2, 'BTC',  100.00000000, 0), (2, 'USDT', 10000000.00000000, 0),
    (2, 'ETH',  500.00000000, 0), (2, 'SOL',     2000.00000000, 0), (2, 'BNB', 300.00000000, 0),
    (3, 'BTC',  100.00000000, 0), (3, 'USDT', 10000000.00000000, 0),
    (3, 'ETH',  500.00000000, 0), (3, 'SOL',     2000.00000000, 0), (3, 'BNB', 300.00000000, 0),
    (4, 'BTC',  100.00000000, 0), (4, 'USDT', 10000000.00000000, 0),
    (4, 'ETH',  500.00000000, 0), (4, 'SOL',     2000.00000000, 0), (4, 'BNB', 300.00000000, 0)
ON CONFLICT (user_id, asset_symbol) DO NOTHING;