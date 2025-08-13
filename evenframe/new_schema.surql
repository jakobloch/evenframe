-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- ACCESSES
-- ------------------------------

DEFINE ACCESS user ON DATABASE TYPE RECORD SIGNUP (CREATE user SET email = $email, password = crypto::argon2::generate($password)) SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password)) WITH JWT ALGORITHM HS512 KEY '9K5WumtGeS3j93LWDTBiQfLIUUTSgCyPuGKVhVo1egSPKl09wsIhn0E8Hy61owuyQl7ZomYtXVDJ6q906CdFQMOyKyIxtn3oMHq9PkuDutQJXf7PlnYhEQcQZVRfKKzP' WITH ISSUER KEY '9K5WumtGeS3j93LWDTBiQfLIUUTSgCyPuGKVhVo1egSPKl09wsIhn0E8Hy61owuyQl7ZomYtXVDJ6q906CdFQMOyKyIxtn3oMHq9PkuDutQJXf7PlnYhEQcQZVRfKKzP' DURATION FOR TOKEN 15m, FOR SESSION 6h;

