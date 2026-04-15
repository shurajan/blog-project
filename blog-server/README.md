## Blog

Rust-проект с PostgreSQL. Миграции применяются автоматически при старте приложения.

## Требования

- Rust (stable)
- Docker и Docker Compose

---

## Переменные окружения

Создай файл `.env` в корне проекта:

```env
DATABASE_URL=postgresql://blog:blog@127.0.0.1:5432/blog
JWT_SECRET=dev_super_secret_change_me_please
```

Для генерации `JWT_SECRET` можно использовать:

```sh
openssl rand -base64 32
```

---

## Запуск базы данных

Поднять PostgreSQL в Docker:

```sh
docker compose up -d
```

Остановить:

```sh
docker compose down
```

Полностью удалить базу вместе с данными:

```sh
docker compose down -v
```

---

## Запуск приложения

```sh
cargo run
```

---

## Полный сброс окружения

```sh
docker compose down -v
docker compose up -d
cargo run
```

---

# Интеграционные тесты

Интеграционные тесты запускают приложение и временную PostgreSQL-базу в Docker через `testcontainers`.

## Требования

- Docker должен быть запущен
- файл `.env` должен существовать

Даже если тесты подменяют `DATABASE_URL`, `AppConfig::from_env()` всё равно читает `.env`, поэтому файл обязателен.

Минимальный `.env`:

```env
DATABASE_URL=postgresql://blog:blog@127.0.0.1:5432/blog
JWT_SECRET=dev_super_secret_change_me_please
```

---

## Запуск всех интеграционных тестов

Рекомендуется запускать тесты последовательно (из-за фиксированных портов):

```sh
cargo test --tests -- --nocapture --test-threads=1
```
---

## Запуск конкретного теста

gRPC:

```sh
cargo test --test grpc_blog_flow -- --nocapture
```

REST:

```sh
cargo test --test rest_blog_flow -- --nocapture
```

---

## Что происходит во время тестов

Каждый интеграционный тест:

1. Поднимает временный контейнер PostgreSQL
2. Запускает приложение (`run_app`)
3. Применяет SQLx миграции
4. Выполняет REST или gRPC сценарий
5. Останавливает приложение
6. Удаляет контейнер

---

## Возможные проблемы

### Docker не запущен

Ошибка:

```
failed to connect to docker daemon
```

Решение: запустить Docker.

---

### Конфликт портов

Тесты используют фиксированные порты (например, `18080`, `15051`).  
При параллельном запуске возможны ошибки.

Решение:

```sh
--test-threads=1
```

---

### Медленный старт PostgreSQL

Первый запуск может занять время (скачивание образа Docker):

```
pulling postgres image...
```

Это нормально.

---

## Примечание

Интеграционные тесты используют изолированную базу через Docker и не влияют на локальную базу из `docker-compose`.
