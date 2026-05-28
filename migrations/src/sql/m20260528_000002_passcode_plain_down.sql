-- 回滚：列名改回 passcode_hash；明文数据保留（不重新 hash，因为缺少加密上下文）。
ALTER TABLE photos RENAME COLUMN passcode TO passcode_hash;
