# Notifico

[📕 Documentation](https://notifico.tech)

Notifico is a self-hosted, open-source notification server that delivers real-time notifications
to various clients via email, SMS, messengers and other means of communication.

## Features

- 📑 **Embedded Templating**: Powerful and flexible templating system powered by [minijinja](https://github.com/mitsuhiko/minijinja), allowing for
  dynamic content generation in notifications.
- ⚡ **Command-line Companion Tool**: [Notificox](https://notifico.tech/notificox), a versatile CLI tool for interacting with Notifico, enabling easy
  notifications from the command line.
- 📈 **Scalable Architecture**: Optimized for high availability configurations using AMQP 1.0 for task distribution.
- 🧑🏻‍💻 **Low-Code implementation**: You can use Notifico just with a little knowledge of JSON. No need for JS or any other language.

## Getting Started

To start using Notifico, please refer to the [📕 Documentation](https://notifico.tech) for detailed installation and
configuration instructions.
The documentation includes guides on setting up the server, configuring different transports, and managing
notifications effectively.

## Notificox

**Notificox** (*Notifico eXecute*) is a command-line tool for interacting with Notifico and running notification pipelines locally or using remote
Notifico instance.
It is designed to be small and self-contained. It can be used to send notifications, using simple URL-like credentials to popular services.

## 🎯 Roadmap:

- [x] Admin panel
- [ ] Helm chart
- [ ] Message view tracking and statistics
- [ ] Subscription API for recipients
- [ ] Push support (FCM, APNS)
- [ ] Bounce handling for Emails
- [ ] debounce.io and similar services support
- [ ] Tracking pixel support
- [ ] Link redirector with statistics
- [x] Webhook support
- [ ] Auto-retry for sending failed messages
- [ ] Template and Pipeline versioning

## 🚆 Available Transports:

| Transport                                                | Transport ID |
|----------------------------------------------------------|--------------|
| [SMTP (email)](https://notifico.tech/plugins/smtp/)      | smtp         |
| [SMPP (SMS)](https://notifico.tech/plugins/smpp/)        | smpp         |
| [Gotify](https://notifico.tech/plugins/gotify/)          | gotify       |
| [Ntfy](https://notifico.tech/plugins/ntfy/)              | ntfy         |
| [Pushover](https://notifico.tech/plugins/pushover/)      | pushover     |
| [Slack](https://notifico.tech/plugins/slack/)            | slack        |
| [Telegram](https://notifico.tech/plugins/telegram/)      | telegram     |
| [WhatsApp Business](https://notifico.tech/plugins/waba/) | waba         |
