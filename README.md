# Notifico

[Documentation](https://notifico.tech)

Notifico is a self-hosted, open-source notification server that delivers real-time notifications
to various clients via email, SMS, messengers and other means of communication.

## Features

- High performance
- Optimized for HA configurations. Uses AMQP 1.0 for distributing tasks between workers.
- List-Unsubscribe support

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
- [ ] Grafana Webhook support

## 🚆 Transports:

- [x] SMTP (email)
- [x] SMPP (SMS)
- [x] Slack
- [x] Telegram
- [x] WhatsApp Business
- [ ] Microsoft Teams
- [ ] Discord
- [ ] Mattermost
- [ ] Twilio
