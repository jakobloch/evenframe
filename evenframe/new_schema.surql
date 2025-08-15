-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- ACCESSES
-- ------------------------------

DEFINE ACCESS user ON DATABASE TYPE RECORD SIGNUP (CREATE user SET email = $email, password = crypto::argon2::generate($password)) SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password)) WITH JWT ALGORITHM HS512 KEY 'EodcqQe2oztHVLzUtU99fTJijS1w19VEVPDMbGsQNmf0mlYMlc6FRJ8FNGqUojSJXZD61jak1kxN3BU1qRmOOICalHeunWHQHwbMcGuJ1AyAIJEUzirKkXuce2wMpH5O' WITH ISSUER KEY 'EodcqQe2oztHVLzUtU99fTJijS1w19VEVPDMbGsQNmf0mlYMlc6FRJ8FNGqUojSJXZD61jak1kxN3BU1qRmOOICalHeunWHQHwbMcGuJ1AyAIJEUzirKkXuce2wMpH5O' DURATION FOR TOKEN 15m, FOR SESSION 6h;

