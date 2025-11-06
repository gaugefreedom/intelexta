# Intelexta Deployment Guide
## Google Cloud Ecosystem Deployment for Google for Startups Cloud Program

This guide covers deploying both applications in the Intelexta alpha release to Google Cloud.

---

## Prerequisites

1. **Google Cloud Account** with billing enabled
2. **Google Cloud CLI (`gcloud`)** installed and authenticated
3. **Firebase CLI** installed: `npm install -g firebase-tools`
4. **Docker** installed (for local testing)
5. **Node.js 20** installed

---

## Part 1: Deploy ChatGPT App (Verifiable Summary Server) to Cloud Run

### Step 1: Set Up Google Cloud Project

```bash
# Set your project ID
export PROJECT_ID="your-project-id"
gcloud config set project $PROJECT_ID

# Enable required APIs
gcloud services enable cloudbuild.googleapis.com
gcloud services enable run.googleapis.com
gcloud services enable containerregistry.googleapis.com
```

### Step 2: Build and Test Docker Image Locally (Optional)

```bash
cd apps/verifiable-summary/server

# Build the Docker image
docker build -t intelexta-verifiable-summary:latest .

# Test locally
docker run -p 8080:8080 \
  -e PORT=8080 \
  -e NODE_ENV=production \
  --user 65532:65532 \
  intelexta-verifiable-summary:latest

# Test the endpoint
curl http://localhost:8080/health
```

> **Operator note:** The runtime image now creates an unprivileged `app` user and runs the service as that account. If you need to override the user when running the container manually, use the UID/GID shown above (Cloud Run uses the container default automatically).

### Step 3: Deploy to Cloud Run

```bash
cd apps/verifiable-summary/server

# Build and deploy in one command
gcloud run deploy intelexta-verifiable-summary \
  --source . \
  --platform managed \
  --region us-central1 \
  --allow-unauthenticated \
  --memory 512Mi \
  --cpu 1 \
  --timeout 60s \
  --max-instances 10 \
  --port 8080

# Or build with Cloud Build and deploy
gcloud builds submit --tag gcr.io/$PROJECT_ID/intelexta-verifiable-summary
gcloud run deploy intelexta-verifiable-summary \
  --image gcr.io/$PROJECT_ID/intelexta-verifiable-summary \
  --platform managed \
  --region us-central1 \
  --allow-unauthenticated \
  --memory 512Mi \
  --cpu 1
```

### Step 4: Set Environment Variables (if needed)

```bash
# Set environment variables for your Cloud Run service
gcloud run services update intelexta-verifiable-summary \
  --region us-central1 \
  --set-env-vars "NODE_ENV=production,LOG_LEVEL=info"

# Or set secrets
gcloud run services update intelexta-verifiable-summary \
  --region us-central1 \
  --update-secrets=API_KEY=your-secret:latest
```

### Step 5: Get Service URL

```bash
gcloud run services describe intelexta-verifiable-summary \
  --region us-central1 \
  --format 'value(status.url)'
```

**Your ChatGPT App is now deployed!** 🎉

The service URL will look like: `https://intelexta-verifiable-summary-xxxxx-uc.a.run.app`

---

## Part 2: Deploy Web Verifier to Firebase Hosting

### Step 1: Set Up Firebase Project

```bash
# Login to Firebase
firebase login

# Initialize Firebase in the web-verifier directory
cd apps/web-verifier
firebase init hosting

# When prompted:
# - Select "Use an existing project" or "Create a new project"
# - Choose your project
# - For "public directory", enter: dist
# - Configure as single-page app: Yes
# - Set up automatic builds with GitHub: No (optional)
# - Overwrite index.html: No
```

**Note:** The `firebase init` command will update `.firebaserc` with your project ID.

### Step 2: Build the Web Verifier

```bash
cd apps/web-verifier

# Build the WASM module first
npm run build:wasm

# Build the React app
npm run build

# Verify the dist folder has content
ls -la dist/
```

### Step 3: Preview Locally (Optional)

```bash
firebase serve --only hosting

# Open http://localhost:5000 to test
```

### Step 4: Deploy to Firebase Hosting

```bash
# Deploy to Firebase Hosting
firebase deploy --only hosting

# Or deploy with a specific project
firebase deploy --only hosting --project your-project-id
```

### Step 5: Get Hosting URL

After deployment completes, Firebase will show you the hosting URL:
```
✔  Deploy complete!

Project Console: https://console.firebase.google.com/project/your-project-id/overview
Hosting URL: https://your-project-id.web.app
```

**Your Web Verifier is now live!** 🎉

---

## Part 3: Update Firebase Configuration

If you need to update your Firebase project ID after initial setup:

1. Edit `apps/web-verifier/.firebaserc`:
   ```json
   {
     "projects": {
       "default": "your-actual-project-id"
     }
   }
   ```

2. Or use Firebase CLI:
   ```bash
   firebase use --add
   # Select your project and give it an alias
   ```

---

## Monitoring and Maintenance

### Cloud Run Monitoring

```bash
# View logs
gcloud run services logs read intelexta-verifiable-summary \
  --region us-central1 \
  --limit 100

# View metrics
gcloud run services describe intelexta-verifiable-summary \
  --region us-central1
```

### Firebase Hosting Monitoring

```bash
# View deployment history
firebase hosting:channel:list

# Rollback to previous version
firebase hosting:clone SITE_ID:SOURCE_CHANNEL SITE_ID:DESTINATION_CHANNEL
```

---

## Cost Optimization

### Cloud Run
- **Free tier**: 2 million requests/month
- **Scaling**: Set `--min-instances 0` for zero cold starts
- **Memory**: Start with 512Mi, adjust based on metrics

### Firebase Hosting
- **Free tier**: 10 GB storage, 360 MB/day transfer
- **Caching**: Enabled automatically with firebase.json config

---

## Troubleshooting

### Cloud Run Issues

**Container fails to start:**
```bash
# Check logs
gcloud run services logs read intelexta-verifiable-summary --region us-central1

# Common issues:
# 1. Port mismatch - ensure app listens on process.env.PORT
# 2. Missing dependencies - check package.json
# 3. Build errors - test Docker build locally first
```

**Health check failures:**
```bash
# Update health check endpoint
gcloud run services update intelexta-verifiable-summary \
  --region us-central1 \
  --no-use-http2
```

### Firebase Hosting Issues

**404 errors on refresh:**
- Already handled by the `rewrites` rule in firebase.json

**WASM not loading:**
- Check CORS headers in firebase.json
- Verify `Content-Type: application/wasm` header is set

**Build artifacts missing:**
```bash
# Rebuild everything
npm run build:wasm
npm run build
firebase deploy --only hosting
```

---

## Security Considerations

### Cloud Run
1. **Authentication**: Currently `--allow-unauthenticated`, change to `--no-allow-unauthenticated` for private services
2. **Secrets**: Use Secret Manager, not environment variables for sensitive data
3. **VPC**: Consider VPC connector for private resources

### Firebase Hosting
1. **Security Rules**: Configure in Firebase Console
2. **SSL**: Enabled by default
3. **Custom Domain**: Add via Firebase Console

---

## CI/CD Integration (Optional)

### Cloud Run with GitHub Actions

Create `.github/workflows/deploy-cloud-run.yml`:
```yaml
name: Deploy to Cloud Run

on:
  push:
    branches: [main]
    paths:
      - 'apps/verifiable-summary/server/**'

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: google-github-actions/setup-gcloud@v0
        with:
          service_account_key: ${{ secrets.GCP_SA_KEY }}
          project_id: ${{ secrets.GCP_PROJECT_ID }}

      - name: Build and Deploy
        run: |
          cd apps/verifiable-summary/server
          gcloud run deploy intelexta-verifiable-summary \
            --source . \
            --region us-central1
```

### Firebase Hosting with GitHub Actions

Create `.github/workflows/deploy-firebase.yml`:
```yaml
name: Deploy to Firebase Hosting

on:
  push:
    branches: [main]
    paths:
      - 'apps/web-verifier/**'

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: 20

      - name: Build
        run: |
          cd apps/web-verifier
          npm ci
          npm run build:wasm
          npm run build

      - uses: FirebaseExtended/action-hosting-deploy@v0
        with:
          repoToken: '${{ secrets.GITHUB_TOKEN }}'
          firebaseServiceAccount: '${{ secrets.FIREBASE_SERVICE_ACCOUNT }}'
          projectId: your-project-id
          channelId: live
```

---

## Additional Resources

- [Cloud Run Documentation](https://cloud.google.com/run/docs)
- [Firebase Hosting Documentation](https://firebase.google.com/docs/hosting)
- [Google for Startups Cloud Program](https://cloud.google.com/startup)

---

## Support

For issues specific to Intelexta deployment, please check:
1. Cloud Run logs: `gcloud run services logs read`
2. Firebase console: https://console.firebase.google.com
3. Build logs: `gcloud builds list`
