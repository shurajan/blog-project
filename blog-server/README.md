# Blog

Rust-проект с PostgreSQL. Миграции применяются автоматически при старте приложения.

## Требования

- Rust (stable)
- Docker и Docker Compose

## Переменные окружения

Создай файл `.env` в корне проекта:
```env
DATABASE_URL=postgresql://blog:blog@127.0.0.1:5432/blog
JWT_SECRET=dev_super_secret_change_me_please
```

для генерации JWT_SECRET можно использовать
```sh
openssl rand -base64 32
```

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

## Запуск приложения
```sh
cargo run
```

## Полный сброс окружения
```sh
docker compose down -v
docker compose up -d
cargo run
```