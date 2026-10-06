# Real-World Metrics

## Examples

Gatekeeper has been deployed in production at the BBAW since mid-2026. Below is an (anonymized) excerpt of our actual metrics. The two domains differ substantially, which directly reflects their distinct implementation strategies.

```JSON
{
    "date": "2026-09-27",
    "domains": {
        "domain_1": {
            "honeypot_hits": 1861321,
            "blocks_no_cookie": 14859,
            "blocks_invalid_cookie": 0,
            "cookie_accepted": 11,
            "verification_successful": 4,
            "verification_failed": 0,
            "good_bot_passes": 143
        },
        "domain_2": {
            "honeypot_hits": 127718,
            "blocks_no_cookie": 550568,
            "blocks_invalid_cookie": 10812,
            "cookie_accepted": 49998,
            "verification_successful": 279,
            "verification_failed": 57,
            "good_bot_passes": 62205
        }
    }
}
```

**domain_1** hosts an object collection repository. Because we rely heavily on caching here, we can limit Gatekeeper (as originally intended) to dynamic pages—specifically faceted search. Consequently, we only observe search users here. Search engine bots also rarely appear because most resources are excluded via `robots.txt`.  
The absurdly high number of honeypot hits is a direct result of our defense strategy. Initially, bots concentrated heavily on search endpoints. Once they encountered static, identical responses there, they migrated to the honeypot. We previously observed those nearly 2 million requests daily hitting our search backend. In the honeypot, they fizzle out harmlessly. Because we force each bot to incur at least 1 second of additional dwell time per request, 1.8M hits consume over 21 days of cumulative crawl time that scrapers cannot dedicate to attacking other resources.

On **domain_2** (a dictionary project), Gatekeeper operates directly at the root level. This does not adhere to the recommended strategy, as all static assets are forced through the `auth_request` loop. However, this situation called for a quick-and-dirty approach, which the metrics validate. Noticeable here are the over 10k invalid cookies. Combined with the relatively high volume of accepted cookies for our baseline, this strongly suggests a two-tier scraper pipeline in operation: an expensive headless browser successfully solved the challenge to obtain a cookie and subsequently attempted to distribute it to a cheaper bulk scraping network. Without dedicated matching configurations, this approach is doomed to fail because TLS handshakes and request headers differ between the two tiers.

## Stability

Across more than 100 days of continuous operation, Gatekeeper has experienced zero outages, and all protected sites have operated with significantly greater stability. The small amount of negative user feedback we received concerned either the visual styling of the verification form (since redesigned) or rare verification failures caused by extremely unstable connections (e.g., trains with high packet loss). However, if bandwidth or latency is insufficient even to load a form without images, modern web applications cannot be used reliably anyway.

## Performance

For a synthetic benchmark, see the [performance audit](./reports/performance-audit_2026-06-22.md). However, these numbers tell only part of the story. At first glance, Gatekeeper increases the load on the proxy server (due to subrequests from `auth_request` and HMAC calculations). Yet, because NGINX handles bot traffic much faster than upstream application servers, the net system load drops substantially. Ultimately, the performance gain depends primarily on the resource intensity of the protected backend. The slower and more compute-heavy the backend, and the higher the proportion of bot traffic relative to legitimate users, the higher the net performance win. In the end, the critical question is whether the infrastructure remains usable under load—and that is precisely what Gatekeeper delivers in our case.