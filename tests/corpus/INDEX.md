# AI-drafted schema corpus

Evaluated 2026-08-07 against noson 0.1.5 and jsonschema 0.49.6. Schemas 00-29 are the first harvest (2026-08-07, model `gemini-3.5-flash`); 30-46 are the distinct schemas from a second harvest the same day (drafts that differed only in `description` strings were dropped).
Each schema was sampled with `noson::generate` using `StdRng::seed_from_u64(seed)`
for seeds `0..100`, and every sample was validated with the consuming service's `jsonschema`
configuration: `should_validate_formats(true)`, unknown formats rejected.

`gen errors` counts seeds where noson returned `Err`; `invalid` counts seeds
where the generated value does not validate against the schema. Suspected gaps
come from a static scan for keywords noson documents as unsupported — the
counts are the ground truth.

| status | file | identifier | gen errors | invalid | suspected gaps |
|---|---|---|--:|--:|---|
| FAIL | [00-emotions.json](00-emotions.json) | emotions | 0/100 | 49/100 | additionalProperties(false), maxProperties |
| ok | [01-paywall_tier.json](01-paywall_tier.json) | paywall_tier | 0/100 | 0/100 | — |
| ok | [02-reading_time.json](02-reading_time.json) | reading_time | 0/100 | 0/100 | — |
| FAIL | [03-topic_tags.json](03-topic_tags.json) | topic_tags | 0/100 | 95/100 | pattern, uniqueItems |
| FAIL | [04-brand_color.json](04-brand_color.json) | brand_color | 0/100 | 100/100 | pattern |
| FAIL | [05-price_ore.json](05-price_ore.json) | price_ore | 0/100 | 94/100 | multipleOf |
| FAIL | [06-review_score.json](06-review_score.json) | review_score | 0/100 | 80/100 | type(array) |
| FAIL | [07-teaser_media.json](07-teaser_media.json) | teaser_media | 0/100 | 100/100 | additionalProperties(false), format:uri |
| ok | [08-site_visibility.json](08-site_visibility.json) | site_visibility | 0/100 | 0/100 | additionalProperties(false), patternProperties |
| FAIL | [09-geo_point.json](09-geo_point.json) | geo_point | 0/100 | 92/100 | prefixItems |
| FAIL | [10-publish_window.json](10-publish_window.json) | publish_window | 0/100 | 23/100 | additionalProperties(false), dependentRequired |
| ok | [11-embedding.json](11-embedding.json) | embedding | 0/100 | 0/100 | — |
| FAIL | [12-author_slug.json](12-author_slug.json) | author_slug | 0/100 | 87/100 | pattern |
| FAIL | [13-content_warnings.json](13-content_warnings.json) | content_warnings | 0/100 | 27/100 | uniqueItems |
| FAIL | [14-ab_variants.json](14-ab_variants.json) | ab_variants | 0/100 | 100/100 | additionalProperties(schema), maxProperties, minProperties |
| FAIL | [15-related_content.json](15-related_content.json) | related_content | 0/100 | 100/100 | pattern, uniqueItems |
| ok | [16-sentiment.json](16-sentiment.json) | sentiment | 0/100 | 0/100 | — |
| ok | [17-push_message.json](17-push_message.json) | push_message | 0/100 | 0/100 | additionalProperties(false), else, if, then |
| ok | [18-external_refs.json](18-external_refs.json) | external_refs | 0/100 | 0/100 | additionalProperties(false), patternProperties |
| ok | [19-featured_until.json](19-featured_until.json) | featured_until | 0/100 | 0/100 | — |
| ok | [20-age_limit.json](20-age_limit.json) | age_limit | 0/100 | 0/100 | — |
| ok | [21-seo_meta.json](21-seo_meta.json) | seo_meta | 0/100 | 0/100 | additionalProperties(false) |
| ok | [22-video_meta.json](22-video_meta.json) | video_meta | 0/100 | 0/100 | additionalProperties(false) |
| FAIL | [23-locale.json](23-locale.json) | locale | 0/100 | 96/100 | pattern |
| ok | [24-corrections.json](24-corrections.json) | corrections | 0/100 | 0/100 | — |
| ok | [25-temperature.json](25-temperature.json) | temperature | 0/100 | 0/100 | — |
| FAIL | [26-contact_email.json](26-contact_email.json) | contact_email | 0/100 | 100/100 | format:email |
| FAIL | [27-canonical_url.json](27-canonical_url.json) | canonical_url | 0/100 | 100/100 | format:uri, pattern |
| FAIL | [28-story_uuid.json](28-story_uuid.json) | story_uuid | 0/100 | 100/100 | format:uuid |
| ok | [29-comment_policy.json](29-comment_policy.json) | comment_policy | 0/100 | 0/100 | additionalProperties(false) |
| FAIL | [30-emotions.json](30-emotions.json) | emotions | 0/100 | 76/100 | additionalProperties(false), maxProperties |
| ok | [31-paywall_tier.json](31-paywall_tier.json) | paywall_tier | 0/100 | 0/100 | — |
| FAIL | [32-topic_tags.json](32-topic_tags.json) | topic_tags | 0/100 | 95/100 | pattern, uniqueItems |
| FAIL | [33-review_score.json](33-review_score.json) | review_score | 0/100 | 80/100 | type(array) |
| FAIL | [34-teaser_media.json](34-teaser_media.json) | teaser_media | 0/100 | 100/100 | additionalProperties(false), format:uri |
| FAIL | [35-geo_point.json](35-geo_point.json) | geo_point | 0/100 | 92/100 | prefixItems |
| ok | [36-embedding.json](36-embedding.json) | embedding | 0/100 | 0/100 | — |
| FAIL | [37-author_slug.json](37-author_slug.json) | author_slug | 0/100 | 87/100 | pattern |
| FAIL | [38-related_content.json](38-related_content.json) | related_content | 0/100 | 100/100 | pattern, uniqueItems |
| ok | [39-sentiment.json](39-sentiment.json) | sentiment | 0/100 | 0/100 | — |
| FAIL | [40-push_message.json](40-push_message.json) | push_message | 0/100 | 93/100 | additionalProperties(false) |
| ok | [41-external_refs.json](41-external_refs.json) | external_refs | 0/100 | 0/100 | additionalProperties(schema), pattern, propertyNames |
| ok | [42-age_limit.json](42-age_limit.json) | age_limit | 0/100 | 0/100 | — |
| ok | [43-video_meta.json](43-video_meta.json) | video_meta | 0/100 | 0/100 | — |
| FAIL | [44-locale.json](44-locale.json) | locale | 0/100 | 91/100 | pattern |
| ok | [45-corrections.json](45-corrections.json) | corrections | 0/100 | 0/100 | additionalProperties(false) |
| ok | [46-temperature.json](46-temperature.json) | temperature | 0/100 | 0/100 | — |
