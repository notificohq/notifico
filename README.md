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

## 🚆 Transports:

- [x] SMTP (email)
- [x] SMPP (SMS)
- [x] Slack
- [x] Telegram
- [x] WhatsApp Business
- [x] Pushover
- [x] Gotify
- [x] Ntfy
- [ ] Apprise API
- [ ] Microsoft Teams
- [ ] Discord
- [ ] Mattermost
- [ ] Twilio
