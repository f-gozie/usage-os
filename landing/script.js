/**
 * UsageOS Landing Page
 * Scroll animations and interactions
 */

(function() {
  'use strict';

  // ==========================================================================
  // Scroll Reveal Animation
  // ==========================================================================

  const observerOptions = {
    root: null,
    rootMargin: '0px 0px -80px 0px',
    threshold: 0.1
  };

  const revealOnScroll = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        entry.target.classList.add('visible');
        // Optionally unobserve after revealing
        // revealOnScroll.unobserve(entry.target);
      }
    });
  }, observerOptions);

  // Elements to animate on scroll
  const animatedElements = document.querySelectorAll(
    '.feature-card, .privacy__content, .privacy__visual, .download__container'
  );

  animatedElements.forEach(el => {
    revealOnScroll.observe(el);
  });

  // ==========================================================================
  // Typing Animation for Terminal
  // ==========================================================================

  const typingElement = document.querySelector('[data-typing]');

  if (typingElement) {
    const text = typingElement.textContent;
    typingElement.textContent = '';
    typingElement.style.borderRight = '2px solid var(--accent-cyan)';

    let i = 0;
    const typeSpeed = 50;

    function typeWriter() {
      if (i < text.length) {
        typingElement.textContent += text.charAt(i);
        i++;
        setTimeout(typeWriter, typeSpeed);
      } else {
        // Remove cursor after typing is done
        setTimeout(() => {
          typingElement.style.borderRight = 'none';
        }, 500);
      }
    }

    // Start typing after a short delay
    setTimeout(typeWriter, 800);
  }

  // ==========================================================================
  // Smooth Scroll for Navigation Links
  // ==========================================================================

  document.querySelectorAll('a[href^="#"]').forEach(anchor => {
    anchor.addEventListener('click', function(e) {
      const targetId = this.getAttribute('href');

      // Skip if it's just "#"
      if (targetId === '#') return;

      const targetElement = document.querySelector(targetId);

      if (targetElement) {
        e.preventDefault();

        targetElement.scrollIntoView({
          behavior: 'smooth',
          block: 'start'
        });
      }
    });
  });

  // ==========================================================================
  // Navigation Background on Scroll
  // ==========================================================================

  const nav = document.querySelector('.nav');
  let lastScrollY = 0;
  let ticking = false;

  function updateNav() {
    const scrollY = window.scrollY;

    if (scrollY > 100) {
      nav.style.background = 'rgba(10, 10, 15, 0.9)';
      nav.style.backdropFilter = 'blur(12px)';
      nav.style.borderBottom = '1px solid rgba(42, 42, 58, 0.5)';
    } else {
      nav.style.background = 'linear-gradient(to bottom, rgba(10, 10, 15, 1) 0%, transparent 100%)';
      nav.style.backdropFilter = 'none';
      nav.style.borderBottom = 'none';
    }

    ticking = false;
  }

  window.addEventListener('scroll', () => {
    lastScrollY = window.scrollY;

    if (!ticking) {
      window.requestAnimationFrame(() => {
        updateNav();
        ticking = false;
      });
      ticking = true;
    }
  });

  // ==========================================================================
  // Parallax Effect for Background Glows
  // ==========================================================================

  const glows = document.querySelectorAll('.bg-glow');

  window.addEventListener('mousemove', (e) => {
    const x = e.clientX / window.innerWidth;
    const y = e.clientY / window.innerHeight;

    glows.forEach((glow, index) => {
      const speed = index === 0 ? 30 : 20;
      const xOffset = (x - 0.5) * speed;
      const yOffset = (y - 0.5) * speed;

      glow.style.transform = `translate(${xOffset}px, ${yOffset}px)`;
    });
  });

  // ==========================================================================
  // Button Ripple Effect
  // ==========================================================================

  document.querySelectorAll('.btn--primary').forEach(button => {
    button.addEventListener('click', function(e) {
      const rect = this.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;

      const ripple = document.createElement('span');
      ripple.style.cssText = `
        position: absolute;
        width: 0;
        height: 0;
        border-radius: 50%;
        background: rgba(255, 255, 255, 0.4);
        transform: translate(-50%, -50%);
        pointer-events: none;
        animation: ripple 0.6s ease-out forwards;
      `;
      ripple.style.left = x + 'px';
      ripple.style.top = y + 'px';

      this.style.position = 'relative';
      this.style.overflow = 'hidden';
      this.appendChild(ripple);

      setTimeout(() => ripple.remove(), 600);
    });
  });

  // Add ripple animation
  const style = document.createElement('style');
  style.textContent = `
    @keyframes ripple {
      to {
        width: 300px;
        height: 300px;
        opacity: 0;
      }
    }
  `;
  document.head.appendChild(style);

  // ==========================================================================
  // Console Easter Egg
  // ==========================================================================

  console.log(
    '%c◈ UsageOS',
    'font-size: 24px; font-weight: bold; color: #00ffd1; text-shadow: 0 0 10px #00ffd1;'
  );
  console.log(
    '%cYour time. Your data. Your machine.',
    'font-size: 14px; color: #8888a0;'
  );
  console.log(
    '%c→ https://github.com/your-repo/usage-os',
    'font-size: 12px; color: #a855f7;'
  );

})();
