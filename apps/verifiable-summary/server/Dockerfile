# Multi-stage Dockerfile for Intelexta Verifiable Summary Server
# Optimized for Google Cloud Run deployment

# ============================================================================
# Stage 1: Build Stage
# ============================================================================
FROM node:20 AS builder

# Set working directory
WORKDIR /app

# Copy package files
COPY package*.json ./

# Install all dependencies (including devDependencies for build)
RUN npm install

# Copy source code and configuration
COPY . .

# Build TypeScript to JavaScript
RUN npm run build

# ============================================================================
# Stage 2: Production Stage
# ============================================================================
FROM node:20-slim

# Set environment to production
ENV NODE_ENV=production

# Set working directory
WORKDIR /app

# Copy package files AND the lock file from the builder stage
COPY --from=builder /app/package.json ./
COPY --from=builder /app/package-lock.json ./

# Install only production dependencies using the lock file
RUN npm ci --omit=dev

# Create non-root user and group for running the service
RUN groupadd --system app && \
    useradd --system --create-home --gid app app

# Copy built application from builder stage
COPY --from=builder --chown=app:app /app/dist ./dist

# Copy any necessary runtime files
# Note: .env files should be set via Cloud Run environment variables
COPY --from=builder --chown=app:app /app/.env.example ./.env.example

# Ensure working directory ownership for runtime operations
RUN chown -R app:app /app

# Expose port (Cloud Run will override this with PORT env var)
# The application should listen on process.env.PORT || 8080
EXPOSE 8080

# Health check (optional but recommended for Cloud Run)
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD node -e "require('http').get('http://localhost:' + (process.env.PORT || 8080) + '/health', (r) => process.exit(r.statusCode === 200 ? 0 : 1))"

# Start the application
# Cloud Run sets the PORT environment variable automatically
USER app
CMD ["node", "dist/index.js"]
