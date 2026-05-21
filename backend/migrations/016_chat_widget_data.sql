-- Add widget metadata to chat_messages for persisting tool result cards
ALTER TABLE chat_messages ADD COLUMN IF NOT EXISTS widget_type TEXT;
ALTER TABLE chat_messages ADD COLUMN IF NOT EXISTS widget_data JSONB;
