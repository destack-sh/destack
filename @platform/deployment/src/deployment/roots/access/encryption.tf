# encrypt the state and plans, which keep the root's access keys, under the state key in KMS
terraform {
  encryption {
    key_provider "aws_kms" "state" {
      kms_key_id = "alias/destack-state"
      region     = "eu-central-1"
      key_spec   = "AES_256"
    }
    method "aes_gcm" "state" {
      keys = key_provider.aws_kms.state
    }
    state {
      method   = method.aes_gcm.state
      enforced = true
    }
    plan {
      method   = method.aes_gcm.state
      enforced = true
    }
  }
}
