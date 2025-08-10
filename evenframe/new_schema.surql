-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- ACCESSES
-- ------------------------------

DEFINE ACCESS user ON DATABASE TYPE RECORD SIGNUP (CREATE user SET email = $email, password = crypto::argon2::generate($password)) SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password)) WITH JWT ALGORITHM HS512 KEY 'ql1RWk25Y1yjgt7vwUKLDl2Rsib6iUOuaYq1ANqB7D5LI95kfAI5SUEm4mYdCDNIlkR7Bynag8jKQESwonPjmRTsKoGfx8tZk6w9vjRtIhFSoyw1Cub1pBtLAQlu7V7M' WITH ISSUER KEY 'ql1RWk25Y1yjgt7vwUKLDl2Rsib6iUOuaYq1ANqB7D5LI95kfAI5SUEm4mYdCDNIlkR7Bynag8jKQESwonPjmRTsKoGfx8tZk6w9vjRtIhFSoyw1Cub1pBtLAQlu7V7M' DURATION FOR TOKEN 15m, FOR SESSION 6h;

