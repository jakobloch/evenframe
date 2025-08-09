-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- ACCESSES
-- ------------------------------

DEFINE ACCESS user ON DATABASE TYPE RECORD SIGNUP (CREATE user SET email = $email, password = crypto::argon2::generate($password)) SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password)) WITH JWT ALGORITHM HS512 KEY 'ZLawPhQ6sQfIeQIEIDvREr8UgAtUyBipcsfqhxn6UkIhygrInDQiBG4dGe52Zqf34P6JIgS3kkCthkm2oejGFte6fvPsAjd1Ivd8LhlricE3NRIZPRnwvgxz15ckLvbD' WITH ISSUER KEY 'ZLawPhQ6sQfIeQIEIDvREr8UgAtUyBipcsfqhxn6UkIhygrInDQiBG4dGe52Zqf34P6JIgS3kkCthkm2oejGFte6fvPsAjd1Ivd8LhlricE3NRIZPRnwvgxz15ckLvbD' DURATION FOR TOKEN 15m, FOR SESSION 6h;

