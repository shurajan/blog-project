-- =====================================================================
-- Blog Server — Database Cleanup Script
-- Run before integration tests to start from a clean state
--
-- Usage:
--   psql "postgresql://blog:blog@127.0.0.1:5432/blog" -f cleanup_db.sql
--
-- Or from psql prompt:
--   \i /path/to/cleanup_db.sql
-- =====================================================================

-- Удаляем посты и пользователей.
-- Порядок важен: сначала posts (FK → users), потом users.
-- TRUNCATE ... CASCADE делает то же самое одной командой,
-- но явный порядок нагляднее.

TRUNCATE TABLE posts RESTART IDENTITY CASCADE;
TRUNCATE TABLE users RESTART IDENTITY CASCADE;
