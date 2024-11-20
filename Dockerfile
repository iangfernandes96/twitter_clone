# Use the official Scylla image
FROM scylladb/scylla:latest

# Install the monitoring agent
RUN apt-get update && \
    apt-get install -y scylla-monitoring-agent

# Set the Scylla Monitoring agent to run on startup
CMD ["scylla", "--seeds=scylla-node1,scylla-node2", "--smp", "2", "--memory", "4G", "--overprovisioned", "1", "--api-address", "0.0.0.0"]
