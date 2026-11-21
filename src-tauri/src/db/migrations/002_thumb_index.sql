-- The thumbnail worker looks for files whose thumbnail is still pending.
CREATE INDEX files_thumb_status ON files(thumb_status);
