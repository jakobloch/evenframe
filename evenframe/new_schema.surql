-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- ACCESSES
-- ------------------------------

DEFINE ACCESS user ON DATABASE TYPE RECORD SIGNUP (CREATE user SET email = $email, password = crypto::argon2::generate($password)) SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password)) WITH JWT ALGORITHM HS512 KEY 'JjdANlGAiDCSLX3UTvWEdDBujwbcVVMXJj6uHCSFreciWYtr9eJnpG53OLK2Y3AvQAQRCL5RRSV8C1feIvkIHg9ZsWeLFnN9itCEaUpYFPvMd55ZGodFylNEgRD8Alpe' WITH ISSUER KEY 'JjdANlGAiDCSLX3UTvWEdDBujwbcVVMXJj6uHCSFreciWYtr9eJnpG53OLK2Y3AvQAQRCL5RRSV8C1feIvkIHg9ZsWeLFnN9itCEaUpYFPvMd55ZGodFylNEgRD8Alpe' DURATION FOR TOKEN 15m, FOR SESSION 6h;

