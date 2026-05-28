-- M2-1: photos.passcode_hash → passcode（明文）
-- 业务原因：passcode 是"分享给朋友的小口令"，不是用户登录密码；
-- admin 需要在后台看到明文以便重新分享给朋友。
ALTER TABLE photos RENAME COLUMN passcode_hash TO passcode;

-- 已存在的 locked 照片：原 hash 无法反解，统一重置为 '1234'（旧 demo 默认值）。
-- 用户首次上去后可在 admin 改成自己想要的。
UPDATE photos SET passcode = '1234' WHERE privacy = 'locked';
UPDATE photos SET passcode = NULL  WHERE privacy <> 'locked';
