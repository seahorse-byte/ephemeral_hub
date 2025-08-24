### Deploy to Netlify (self hosting)

```bash
pwd # ....<local_computer_path>/ephemeral_spaces
```

```bash
cd target/dx/ephemeral_web/release/web/public
```

```bash
npx netlify-cli deploy --auth "$NETLIFY_AUTH_TOKEN" --site "$NETLIFY_SITE_ID" --prod
```
