# AI-drafted schema corpus

Evaluated 2026-08-07 against noson 0.2.0 and jsonschema 0.49.6. Files 00-46 are the harvests that drove noson 0.2.0 (all green since); 47-73 are distinct schemas from three later harvests, including entries baiting the remaining unsupported keywords (contains, not, dependentSchemas, patternProperties enforcement).
Each schema was sampled with `noson::generate` using `StdRng::seed_from_u64(seed)`
for seeds `0..1000`, and every sample was validated with the consuming service's `jsonschema`
configuration: `should_validate_formats(true)`, unknown formats rejected.

`gen errors` counts seeds where noson returned `Err`; `invalid` counts seeds
where the generated value does not validate against the schema. Suspected gaps
come from a static scan for keywords noson documents as unsupported — the
counts are the ground truth.

| status | file | identifier | gen errors | invalid | suspected gaps |
|---|---|---|--:|--:|---|
| ok | [00-emotions.json](00-emotions.json) | emotions | 0/1000 | 0/1000 | additionalProperties(false), maxProperties |
| ok | [01-paywall_tier.json](01-paywall_tier.json) | paywall_tier | 0/1000 | 0/1000 | — |
| ok | [02-reading_time.json](02-reading_time.json) | reading_time | 0/1000 | 0/1000 | — |
| ok | [03-topic_tags.json](03-topic_tags.json) | topic_tags | 0/1000 | 0/1000 | pattern, uniqueItems |
| ok | [04-brand_color.json](04-brand_color.json) | brand_color | 0/1000 | 0/1000 | pattern |
| ok | [05-price_ore.json](05-price_ore.json) | price_ore | 0/1000 | 0/1000 | multipleOf |
| ok | [06-review_score.json](06-review_score.json) | review_score | 0/1000 | 0/1000 | type(array) |
| ok | [07-teaser_media.json](07-teaser_media.json) | teaser_media | 0/1000 | 0/1000 | additionalProperties(false), format:uri |
| ok | [08-site_visibility.json](08-site_visibility.json) | site_visibility | 0/1000 | 0/1000 | additionalProperties(false), patternProperties |
| ok | [09-geo_point.json](09-geo_point.json) | geo_point | 0/1000 | 0/1000 | prefixItems |
| ok | [10-publish_window.json](10-publish_window.json) | publish_window | 0/1000 | 0/1000 | additionalProperties(false), dependentRequired |
| ok | [11-embedding.json](11-embedding.json) | embedding | 0/1000 | 0/1000 | — |
| ok | [12-author_slug.json](12-author_slug.json) | author_slug | 0/1000 | 0/1000 | pattern |
| ok | [13-content_warnings.json](13-content_warnings.json) | content_warnings | 0/1000 | 0/1000 | uniqueItems |
| ok | [14-ab_variants.json](14-ab_variants.json) | ab_variants | 0/1000 | 0/1000 | additionalProperties(schema), maxProperties, minProperties |
| ok | [15-related_content.json](15-related_content.json) | related_content | 0/1000 | 0/1000 | pattern, uniqueItems |
| ok | [16-sentiment.json](16-sentiment.json) | sentiment | 0/1000 | 0/1000 | — |
| ok | [17-push_message.json](17-push_message.json) | push_message | 0/1000 | 0/1000 | additionalProperties(false), else, if, then |
| ok | [18-external_refs.json](18-external_refs.json) | external_refs | 0/1000 | 0/1000 | additionalProperties(false), patternProperties |
| ok | [19-featured_until.json](19-featured_until.json) | featured_until | 0/1000 | 0/1000 | — |
| ok | [20-age_limit.json](20-age_limit.json) | age_limit | 0/1000 | 0/1000 | — |
| ok | [21-seo_meta.json](21-seo_meta.json) | seo_meta | 0/1000 | 0/1000 | additionalProperties(false) |
| ok | [22-video_meta.json](22-video_meta.json) | video_meta | 0/1000 | 0/1000 | additionalProperties(false) |
| ok | [23-locale.json](23-locale.json) | locale | 0/1000 | 0/1000 | pattern |
| ok | [24-corrections.json](24-corrections.json) | corrections | 0/1000 | 0/1000 | — |
| ok | [25-temperature.json](25-temperature.json) | temperature | 0/1000 | 0/1000 | — |
| ok | [26-contact_email.json](26-contact_email.json) | contact_email | 0/1000 | 0/1000 | format:email |
| ok | [27-canonical_url.json](27-canonical_url.json) | canonical_url | 0/1000 | 0/1000 | format:uri, pattern |
| ok | [28-story_uuid.json](28-story_uuid.json) | story_uuid | 0/1000 | 0/1000 | format:uuid |
| ok | [29-comment_policy.json](29-comment_policy.json) | comment_policy | 0/1000 | 0/1000 | additionalProperties(false) |
| ok | [30-emotions.json](30-emotions.json) | emotions | 0/1000 | 0/1000 | additionalProperties(false), maxProperties |
| ok | [31-paywall_tier.json](31-paywall_tier.json) | paywall_tier | 0/1000 | 0/1000 | — |
| ok | [32-topic_tags.json](32-topic_tags.json) | topic_tags | 0/1000 | 0/1000 | pattern, uniqueItems |
| ok | [33-review_score.json](33-review_score.json) | review_score | 0/1000 | 0/1000 | type(array) |
| ok | [34-teaser_media.json](34-teaser_media.json) | teaser_media | 0/1000 | 0/1000 | additionalProperties(false), format:uri |
| ok | [35-geo_point.json](35-geo_point.json) | geo_point | 0/1000 | 0/1000 | prefixItems |
| ok | [36-embedding.json](36-embedding.json) | embedding | 0/1000 | 0/1000 | — |
| ok | [37-author_slug.json](37-author_slug.json) | author_slug | 0/1000 | 0/1000 | pattern |
| ok | [38-related_content.json](38-related_content.json) | related_content | 0/1000 | 0/1000 | pattern, uniqueItems |
| ok | [39-sentiment.json](39-sentiment.json) | sentiment | 0/1000 | 0/1000 | — |
| ok | [40-push_message.json](40-push_message.json) | push_message | 0/1000 | 0/1000 | additionalProperties(false) |
| ok | [41-external_refs.json](41-external_refs.json) | external_refs | 0/1000 | 0/1000 | additionalProperties(schema), pattern, propertyNames |
| ok | [42-age_limit.json](42-age_limit.json) | age_limit | 0/1000 | 0/1000 | — |
| ok | [43-video_meta.json](43-video_meta.json) | video_meta | 0/1000 | 0/1000 | — |
| ok | [44-locale.json](44-locale.json) | locale | 0/1000 | 0/1000 | pattern |
| ok | [45-corrections.json](45-corrections.json) | corrections | 0/1000 | 0/1000 | additionalProperties(false) |
| ok | [46-temperature.json](46-temperature.json) | temperature | 0/1000 | 0/1000 | — |
| ok | [47-emotions.json](47-emotions.json) | emotions | 0/1000 | 0/1000 | additionalProperties(false), maxProperties |
| ok | [48-topic_tags.json](48-topic_tags.json) | topic_tags | 0/1000 | 0/1000 | pattern, uniqueItems |
| ok | [49-teaser_media.json](49-teaser_media.json) | teaser_media | 0/1000 | 0/1000 | format:uri |
| ok | [50-geo_point.json](50-geo_point.json) | geo_point | 0/1000 | 0/1000 | prefixItems |
| ok | [51-author_slug.json](51-author_slug.json) | author_slug | 0/1000 | 0/1000 | pattern |
| ok | [52-video_meta.json](52-video_meta.json) | video_meta | 0/1000 | 0/1000 | additionalProperties(schema) |
| ok | [53-locale.json](53-locale.json) | locale | 0/1000 | 0/1000 | pattern |
| ok | [54-contact_email.json](54-contact_email.json) | contact_email | 0/1000 | 0/1000 | format:email |
| ok | [55-comment_policy.json](55-comment_policy.json) | comment_policy | 0/1000 | 0/1000 | additionalProperties(false) |
| ok | [56-topic_tags.json](56-topic_tags.json) | topic_tags | 0/1000 | 0/1000 | pattern, uniqueItems |
| ok | [57-brand_color.json](57-brand_color.json) | brand_color | 0/1000 | 0/1000 | pattern |
| ok | [58-teaser_media.json](58-teaser_media.json) | teaser_media | 0/1000 | 0/1000 | additionalProperties(false) |
| ok | [59-geo_point.json](59-geo_point.json) | geo_point | 0/1000 | 0/1000 | prefixItems |
| ok | [60-embedding.json](60-embedding.json) | embedding | 0/1000 | 0/1000 | — |
| ok | [61-external_refs.json](61-external_refs.json) | external_refs | 0/1000 | 0/1000 | additionalProperties(false), patternProperties |
| ok | [62-featured_until.json](62-featured_until.json) | featured_until | 0/1000 | 0/1000 | — |
| ok | [63-locale.json](63-locale.json) | locale | 0/1000 | 0/1000 | pattern |
| FAIL | [64-gallery.json](64-gallery.json) | gallery | 0/1000 | 594/1000 | contains, format:uri |
| ok | [65-discount.json](65-discount.json) | discount | 0/1000 | 0/1000 | additionalProperties(false) |
| ok | [66-custom_meta.json](66-custom_meta.json) | custom_meta | 0/1000 | 0/1000 | additionalProperties(false), patternProperties |
| ok | [67-byline.json](67-byline.json) | byline | 0/1000 | 0/1000 | additionalProperties(false), format:email |
| ok | [68-highlight_quotes.json](68-highlight_quotes.json) | highlight_quotes | 0/1000 | 0/1000 | contains, if, minContains, then |
| ok | [69-emotions.json](69-emotions.json) | emotions | 0/1000 | 0/1000 | additionalProperties(false), maxProperties |
| ok | [70-teaser_media.json](70-teaser_media.json) | teaser_media | 0/1000 | 0/1000 | additionalProperties(false) |
| ok | [71-locale.json](71-locale.json) | locale | 0/1000 | 0/1000 | pattern |
| FAIL | [72-gallery.json](72-gallery.json) | gallery | 0/1000 | 696/1000 | contains, format:uri |
| ok | [73-display_name.json](73-display_name.json) | display_name | 0/1000 | 0/1000 | not |
