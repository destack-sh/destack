# keep the universe's files in Zurich
resource "aws_s3_bucket" "files" {
  bucket = "destack-${var.environment}-files-universe"
  lifecycle {
    prevent_destroy = true
  }
}

# keep versions, which object locks need
resource "aws_s3_bucket_versioning" "files" {
  bucket = aws_s3_bucket.files.id
  versioning_configuration {
    status = "Enabled"
  }
}

# let the universe lock an event segment until its retention ends
resource "aws_s3_bucket_object_lock_configuration" "files" {
  bucket              = aws_s3_bucket.files.id
  object_lock_enabled = "Enabled"
  depends_on          = [aws_s3_bucket_versioning.files]
}

resource "aws_s3_bucket_public_access_block" "files" {
  bucket                  = aws_s3_bucket.files.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_ownership_controls" "files" {
  bucket = aws_s3_bucket.files.id
  rule {
    object_ownership = "BucketOwnerEnforced"
  }
}
