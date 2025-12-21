CREATE TABLE webhook_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    webhook_endpoint_id UUID NOT NULL REFERENCES webhook_endpoints(id),
    transaction_id UUID NOT NULL REFERENCES transactions(id),
    payload JSONB NOT NULL,
    sent_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    response_status INTEGER,
    response_body TEXT
);

CREATE INDEX idx_webhook_events_transaction ON webhook_events(transaction_id);
