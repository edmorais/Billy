-- Annotations are stored as the bytes PDF.js appends to the original, not as full copies.
-- base_len: how many bytes of the original the annotations extend.
-- base_check: hash of the original's last bytes before base_len, to check it is still there.
-- Rows without base_len are full annotated copies (older saves, or saves PDF.js did not
-- make by appending).
ALTER TABLE annotations ADD COLUMN base_len INTEGER;
ALTER TABLE annotations ADD COLUMN base_check TEXT;
