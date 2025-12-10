FROM denoland/deno:alpine-2.1.4

# Set working directory
WORKDIR /app

# Copy application files
COPY . .

# Remove incompatible lockfile and cache dependencies
RUN rm -f deno.lock && deno cache main.ts

# Expose port
EXPOSE 8080

# Run the application
CMD ["run", "--allow-net", "--allow-env", "--allow-read", "--allow-write", "--unstable-kv", "main.ts"]
